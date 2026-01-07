//! Example: Port forwarding with HDC
//!
//! This example demonstrates how to create and manage port forwards.

use hdc_rs::{ForwardNode, HdcClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Connect to HDC server
    let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    println!("✓ Connected to HDC server");

    // List available devices
    let devices = client.list_targets().await?;
    if devices.is_empty() {
        println!("✗ No devices found");
        return Ok(());
    }
    println!("✓ Found {} device(s)", devices.len());

    // Select first device
    client.connect_device(&devices[0]).await?;
    println!("✓ Connected to device: {}", devices[0]);

    // Example 1: Forward local TCP port to device TCP port
    println!("\n=== Example 1: TCP Forward ===");
    println!("Creating forward: local tcp:8080 -> device tcp:8081");
    match client
        .fport(ForwardNode::Tcp(8080), ForwardNode::Tcp(8081))
        .await
    {
        Ok(response) => println!("✓ Forward created: {}", response),
        Err(e) => println!("✗ Failed to create forward: {}", e),
    }

    // Example 2: Reverse forward (device to local)
    println!("\n=== Example 2: Reverse Forward ===");
    println!("Creating reverse forward: device tcp:9090 -> local tcp:9091");
    match client
        .rport(ForwardNode::Tcp(9090), ForwardNode::Tcp(9091))
        .await
    {
        Ok(response) => println!("✓ Reverse forward created: {}", response),
        Err(e) => println!("✗ Failed to create reverse forward: {}", e),
    }

    // Example 3: List all forwards
    println!("\n=== Example 3: List Forwards ===");
    match client.fport_list().await {
        Ok(tasks) => {
            if tasks.is_empty() {
                println!("No forward tasks");
            } else {
                println!("Active forward tasks:");
                for (i, task) in tasks.iter().enumerate() {
                    println!("  {}. {}", i + 1, task);
                }
            }
        }
        Err(e) => println!("✗ Failed to list forwards: {}", e),
    }

    // Example 4: Remove a forward
    println!("\n=== Example 4: Remove Forward ===");
    println!("Removing forward: tcp:8080 tcp:8081");
    match client.fport_remove("tcp:8080 tcp:8081").await {
        Ok(response) => println!("✓ Forward removed: {}", response),
        Err(e) => println!("✗ Failed to remove forward: {}", e),
    }

    // Example 5: Forward to JDWP (Java Debug Wire Protocol)
    println!("\n=== Example 5: JDWP Forward ===");
    println!("Creating JDWP forward: local tcp:8700 -> device jdwp:12345");
    match client
        .fport(ForwardNode::Tcp(8700), ForwardNode::Jdwp(12345))
        .await
    {
        Ok(response) => println!("✓ JDWP forward created: {}", response),
        Err(e) => println!("✗ Failed to create JDWP forward: {}", e),
    }

    println!("\n✓ Examples completed");
    Ok(())
}
