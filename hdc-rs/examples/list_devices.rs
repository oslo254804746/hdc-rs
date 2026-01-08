//! Example: List all connected devices

use hdc_rs::HdcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("hdc_rs=debug,list_devices=info")
        .init();

    println!("HDC Rust Client - List Devices Example");
    println!("========================================\n");

    // Connect to HDC server
    println!("Connecting to HDC server at 127.0.0.1:8710...");
    let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    println!("✓ Connected successfully\n");

    // Check server version
    println!("Checking server version...");
    match client.check_server().await {
        Ok(version) => println!("Server version: {}\n", version),
        Err(e) => println!("Warning: Could not get server version: {}\n", e),
    }

    // List all connected devices
    println!("Listing connected devices...");
    let devices = client.list_targets().await?;

    if devices.is_empty() {
        println!("✗ No devices found");
        println!("\nMake sure:");
        println!("  1. HDC server is running (hdc start)");
        println!("  2. Device is connected via USB or TCP");
        println!("  3. Device is authorized");
    } else {
        println!("✓ Found {} device(s):\n", devices.len());
        for (i, device) in devices.iter().enumerate() {
            println!("  [{}] {}", i + 1, device);
        }
    }

    println!("\nClosing connection...");
    client.close().await?;
    println!("✓ Done");

    Ok(())
}
