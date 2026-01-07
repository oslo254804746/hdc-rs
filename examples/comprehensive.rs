//! Comprehensive example demonstrating all HDC features
//!
//! This example shows how to use shell commands, port forwarding, and app management.

use hdc_rs::{ForwardNode, HdcClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== HDC Rust Client - Comprehensive Example ===\n");

    // Step 1: Connect to HDC server
    println!("Step 1: Connecting to HDC server...");
    let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    println!("✓ Connected\n");

    // Step 2: List and select device
    println!("Step 2: Finding devices...");
    let devices = client.list_targets().await?;
    if devices.is_empty() {
        println!("✗ No devices found. Please connect a device and try again.");
        return Ok(());
    }
    println!("✓ Found {} device(s):", devices.len());
    for (i, device) in devices.iter().enumerate() {
        println!("  {}. {}", i + 1, device);
    }

    let device_id = &devices[0];
    println!("\nStep 3: Selecting device: {}", device_id);
    client.connect_device(device_id).await?;
    println!("✓ Device selected\n");

    // Step 4: Execute shell commands
    println!("Step 4: Executing shell commands...");

    println!("  - Getting device info:");
    match client.shell("getprop ro.product.model").await {
        Ok(output) => println!("    Model: {}", output.trim()),
        Err(e) => println!("    Failed: {}", e),
    }

    println!("  - Checking storage:");
    match client.shell("df -h /data").await {
        Ok(output) => {
            for line in output.lines().take(2) {
                println!("    {}", line);
            }
        }
        Err(e) => println!("    Failed: {}", e),
    }

    // Step 5: Port forwarding
    println!("\nStep 5: Managing port forwards...");

    println!("  - Creating TCP forward (8080 -> 8081):");
    match client
        .fport(ForwardNode::Tcp(8080), ForwardNode::Tcp(8081))
        .await
    {
        Ok(response) => println!("    ✓ {}", response.trim()),
        Err(e) => println!("    ✗ {}", e),
    }

    println!("  - Listing all forwards:");
    match client.fport_list().await {
        Ok(tasks) => {
            if tasks.is_empty() {
                println!("    (none)");
            } else {
                for task in &tasks {
                    println!("    - {}", task);
                }
            }
        }
        Err(e) => println!("    ✗ {}", e),
    }

    println!("  - Removing forward:");
    match client.fport_remove("tcp:8080 tcp:8081").await {
        Ok(response) => println!("    ✓ {}", response.trim()),
        Err(e) => println!("    ✗ {}", e),
    }

    // Step 6: App management demo (informational)
    println!("\nStep 6: App management (demo commands)...");
    println!("  Note: These are example commands. Replace paths with actual files.\n");

    println!("  Install app:");
    println!("    client.install(&[\"app.hap\"], InstallOptions::new().replace(true)).await?;");

    println!("\n  Install shared bundle:");
    println!("    client.install(&[\"shared.hsp\"], InstallOptions::new().shared(true)).await?;");

    println!("\n  Uninstall app:");
    println!("    client.uninstall(\"com.example.app\", UninstallOptions::new()).await?;");

    println!("\n  Uninstall but keep data:");
    println!("    client.uninstall(\"com.example.app\", UninstallOptions::new().keep_data(true)).await?;");

    // Step 7: Device logs (hilog)
    println!("\nStep 7: Reading device logs...");
    println!("  - Getting recent logs (with 2 second timeout):");
    match client.hilog(Some("-x")).await {
        Ok(logs) => {
            let lines: Vec<&str> = logs.lines().take(5).collect();
            if lines.is_empty() {
                println!("    (no logs)");
            } else {
                for line in lines {
                    println!("    {}", line);
                }
                let total_lines = logs.lines().count();
                if total_lines > 5 {
                    println!("    ... ({} more lines)", total_lines - 5);
                }
            }
        }
        Err(e) => println!("    Note: {}", e),
    }

    // Summary
    println!("\n=== Summary ===");
    println!("✓ All operations completed successfully!");
    println!("\nFor more examples, see:");
    println!("  - examples/simple_shell.rs - Shell commands");
    println!("  - examples/forward_demo.rs - Port forwarding");
    println!("  - examples/app_demo.rs - App management");
    println!("  - examples/hilog_demo.rs - Device logs");

    Ok(())
}
