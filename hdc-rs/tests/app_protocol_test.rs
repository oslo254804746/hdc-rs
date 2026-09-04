use std::time::Duration;

use hdc_rs::protocol::{ChannelHandShake, PacketCodec};
use hdc_rs::{HdcClient, HdcError, InstallOptions, UninstallOptions};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

const PACKAGE: &str = "com.example.disposable";

async fn send_handshake(stream: &mut TcpStream, codec: &mut PacketCodec) {
    let mut handshake = ChannelHandShake::default();
    handshake.banner[..8].copy_from_slice(b"OHOS HDC");
    handshake.set_channel_id(1);
    codec
        .write_packet(stream, &handshake.to_bytes_without_version())
        .await
        .unwrap();

    // The client echoes the handshake format before sending its command.
    codec.read_packet(stream).await.unwrap();
}

async fn start_uninstall_server(
    expected_command: &'static [u8],
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut codec = PacketCodec::new();
        send_handshake(&mut stream, &mut codec).await;

        let command = codec.read_packet(&mut stream).await.unwrap();
        assert_eq!(command, expected_command);

        // Multiple data frames must be joined until KernelChannelClose.
        codec
            .write_packet(&mut stream, b"uninstall ")
            .await
            .unwrap();
        codec.write_packet(&mut stream, b"complete").await.unwrap();
        codec.write_packet(&mut stream, &[2, 0]).await.unwrap();

        // The client drops its task channel after consuming the close frame.
        let mut byte = [0u8; 1];
        match stream.read(&mut byte).await {
            Ok(0) | Err(_) => {}
            Ok(size) => panic!("client kept uninstall channel open and sent {size} bytes"),
        }
    });
    (address, server)
}

async fn assert_uninstall_command(options: UninstallOptions, expected_command: &'static [u8]) {
    let (address, server) = start_uninstall_server(expected_command).await;
    let mut client = HdcClient::connect(address).await.unwrap();
    let response = client.uninstall(PACKAGE, options).await.unwrap();

    assert_eq!(response, "uninstall complete");
    assert!(!client.is_connected());
    server.await.unwrap();
}

#[tokio::test]
async fn uninstall_without_options_uses_separate_package_name_argument() {
    assert_uninstall_command(
        UninstallOptions::new(),
        b"uninstall -n com.example.disposable",
    )
    .await;
}

#[tokio::test]
async fn uninstall_keep_data_uses_keep_data_and_package_arguments() {
    assert_uninstall_command(
        UninstallOptions::new().keep_data(true),
        b"uninstall -k -n com.example.disposable",
    )
    .await;
}

struct DelayedInstallServer {
    address: String,
    command_seen: oneshot::Receiver<()>,
    release: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

async fn start_delayed_install_server() -> DelayedInstallServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (command_seen_tx, command_seen) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut codec = PacketCodec::new();
        send_handshake(&mut stream, &mut codec).await;

        let command = codec.read_packet(&mut stream).await.unwrap();
        assert_eq!(command, b"install app.hap");
        command_seen_tx.send(()).unwrap();

        // The test controls when the response arrives, allowing the client's
        // 300 second response-frame timeout to be advanced without wall-clock
        // wait.
        let _ = release_rx.await;
        let _ = codec.write_packet(&mut stream, b"install ").await;
        let _ = codec.write_packet(&mut stream, b"complete").await;
        let _ = codec.write_packet(&mut stream, &[2, 0]).await;
    });

    DelayedInstallServer {
        address,
        command_seen,
        release: release_tx,
        task,
    }
}

async fn wait_for_install_command(
    client: &mut HdcClient,
    command_seen: oneshot::Receiver<()>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = hdc_rs::Result<String>> + '_>> {
    let install = client.install(&["app.hap"], InstallOptions::new());
    let mut install = Box::pin(install);
    let mut command_seen = command_seen;
    tokio::select! {
        result = &mut install => panic!("install returned before mock response: {result:?}"),
        _ = &mut command_seen => {}
    }
    install
}

#[tokio::test]
async fn install_survives_thirty_one_seconds_of_silence() {
    let server = start_delayed_install_server().await;
    let mut client = HdcClient::connect(&server.address).await.unwrap();
    let install = wait_for_install_command(&mut client, server.command_seen).await;

    // Handshake and command IO are complete before virtual time is paused.
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(31)).await;
    tokio::time::resume();
    server.release.send(()).unwrap();

    assert_eq!(install.await.unwrap(), "install complete");
    assert!(!client.is_connected());
    server.task.await.unwrap();
}

#[tokio::test]
async fn install_times_out_after_three_hundred_seconds_and_disconnects() {
    let server = start_delayed_install_server().await;
    let mut client = HdcClient::connect(&server.address).await.unwrap();
    let install = wait_for_install_command(&mut client, server.command_seen).await;

    // Advance past the client's 300 second timeout without waiting in real
    // time. The server is still silent on the already-established channel.
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(301)).await;
    tokio::time::resume();

    assert!(matches!(install.await, Err(HdcError::Timeout)));
    assert!(!client.is_connected());
    // Unblock the server so the test leaves no task behind. Its writes may
    // fail because the client has already closed the channel.
    let _ = server.release.send(());
    server.task.await.unwrap();
}
