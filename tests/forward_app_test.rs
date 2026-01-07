//! Integration tests for forward and app commands

use hdc_rs::{ForwardNode, HdcClient, UninstallOptions};
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
#[ignore] // Requires HDC server and device
async fn test_forward_commands() {
    init_logger();

    let mut client = HdcClient::connect("127.0.0.1:8710")
        .await
        .expect("Failed to connect");

    // Get devices
    let devices = client.list_targets().await.expect("Failed to list devices");
    assert!(!devices.is_empty(), "No devices connected");
    info!("Found device: {}", devices[0]);

    // Connect to device
    client
        .connect_device(&devices[0])
        .await
        .expect("Failed to connect to device");

    // Test 1: Create forward
    info!("Testing forward creation...");
    let result = client
        .fport(ForwardNode::Tcp(18080), ForwardNode::Tcp(18081))
        .await;
    match &result {
        Ok(response) => info!("Forward created: {}", response),
        Err(e) => info!("Forward creation result: {}", e),
    }

    // Test 2: List forwards (requires reconnect as previous command may have closed connection)
    let mut client = HdcClient::connect("127.0.0.1:8710")
        .await
        .expect("Failed to reconnect");
    client
        .connect_device(&devices[0])
        .await
        .expect("Failed to reconnect to device");

    info!("Testing forward list...");
    let tasks = client.fport_list().await;
    match &tasks {
        Ok(tasks) => info!("Forward tasks: {:?}", tasks),
        Err(e) => info!("Forward list result: {}", e),
    }

    // Test 3: Remove forward
    let mut client = HdcClient::connect("127.0.0.1:8710")
        .await
        .expect("Failed to reconnect");
    client
        .connect_device(&devices[0])
        .await
        .expect("Failed to reconnect to device");

    info!("Testing forward removal...");
    let result = client.fport_remove("tcp:18080 tcp:18081").await;
    match &result {
        Ok(response) => info!("Forward removed: {}", response),
        Err(e) => info!("Forward removal result: {}", e),
    }

    // let mut client = HdcClient::connect("127.0.0.1:8710")
    //     .await
    //     .expect("Failed to reconnect");
    // client.connect_device(&devices[0]).await.expect("Failed to reconnect to device");
    //
    // info!("Testing forward removal...");
    // let result = client.fport_remove("tcp:60325 tcp:4567").await;
    // match &result {
    //     Ok(response) => info!("Forward removed: {}", response),
    //     Err(e) => info!("Forward removal result: {}", e),
    // }
}

#[tokio::test]
#[ignore] // Requires HDC server, device, and test app
async fn test_app_commands() {
    init_logger();

    let mut client = HdcClient::connect("127.0.0.1:8710")
        .await
        .expect("Failed to connect");

    let devices = client.list_targets().await.expect("Failed to list devices");
    assert!(!devices.is_empty(), "No devices connected");

    client
        .connect_device(&devices[0])
        .await
        .expect("Failed to connect to device");

    // Note: This test just verifies the API works
    // Actual install/uninstall requires valid .hap files
    info!("App commands API available");
    info!("Use InstallOptions::new().replace(true) for install");
    info!("Use UninstallOptions::new().keep_data(true) for uninstall");
    let uninstall_resp = client
        .uninstall("com.huawei.hmsapp.hismartperf", UninstallOptions::default())
        .await;
    info!("卸载app结果 {:#?}", uninstall_resp)
}
