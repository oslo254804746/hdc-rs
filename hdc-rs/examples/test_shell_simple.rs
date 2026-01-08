//! Simple test for shell command

use hdc_rs::HdcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("hdc_rs=debug")
        .init();

    println!("Testing Shell Command\n");

    // Connect
    let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    println!("✓ Connected to server\n");

    // Get device
    let devices = client.list_targets().await?;
    if devices.is_empty() {
        eprintln!("No devices found");
        return Ok(());
    }
    let device = &devices[0];
    println!("Device: {}\n", device);

    // Connect to device
    client.connect_device(device).await?;
    println!("✓ Connected to device\n");

    // Test a simple command
    println!("Testing: echo hello");
    match client.shell("echo hello").await {
        Ok(output) => {
            println!("Success!");
            println!("Output: '{}'", output.trim());
            println!("Length: {} bytes", output.len());
        }
        Err(e) => {
            eprintln!("Failed: {}", e);
        }
    }

    println!("\nTesting: pwd");
    match client.shell("pwd").await {
        Ok(output) => {
            println!("Success!");
            println!("Output: '{}'", output.trim());
        }
        Err(e) => {
            eprintln!("Failed: {}", e);
        }
    }

    println!("\nDone");
    Ok(())
}
