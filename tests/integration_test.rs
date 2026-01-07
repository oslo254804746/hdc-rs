//! Quick test to verify the client can connect and list devices

use hdc_rs::HdcClient;
use tracing::info;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn init_logger() {
    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_level(true))
        .try_init();
}

#[tokio::test]
#[ignore] // Requires HDC server running
async fn test_connection() {
    let result = HdcClient::connect("127.0.0.1:8710").await;
    assert!(result.is_ok(), "Should connect to HDC server");
}

#[tokio::test]
#[ignore] // Requires HDC server running
async fn test_list_targets() {
    init_logger();
    let mut client = HdcClient::connect("127.0.0.1:8710")
        .await
        .expect("Failed to connect");

    let devices = client.list_targets().await;
    assert!(devices.is_ok(), "Should list targets");
    info!("list device 输出 {:#?}", devices.unwrap());
}

#[tokio::test]
#[ignore] // Requires HDC server running and device connected
async fn test_shell_command() {
    init_logger();
    let mut client = HdcClient::connect("127.0.0.1:8710")
        .await
        .expect("Failed to connect");

    // First, get the list of devices
    let devices = client.list_targets().await.expect("Failed to list devices");
    assert!(
        !devices.is_empty(),
        "No devices connected. Please connect a device first."
    );

    let device_id = &devices[0];
    info!("Using device: {}", device_id);

    // Connect to the first device
    client
        .connect_device(device_id)
        .await
        .expect("Failed to connect to device");

    // Now execute shell command
    let output = client.shell("echo 'hello'").await;
    assert!(output.is_ok(), "Should execute shell command");
    let obj = output.unwrap();
    info!("shell output {:?}", obj);
}

#[tokio::test]
#[ignore] // Requires HDC server running and device connected
async fn test_shell_on_device() {
    init_logger();
    let mut client = HdcClient::connect("127.0.0.1:8710")
        .await
        .expect("Failed to connect");

    // Get device list
    let devices = client.list_targets().await.expect("Failed to list devices");
    assert!(!devices.is_empty(), "No devices connected");

    // Execute command directly on device without connect_device()
    let output = client
        .shell_on_device(&devices[0], "echo 'hello from device'")
        .await;
    assert!(output.is_ok(), "Should execute shell command on device");
    info!("shell_on_device output: {:?}", output.unwrap());
}
