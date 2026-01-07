//! Device Monitor Example
//!
//! This example demonstrates how to monitor device connections and disconnections.
//!
//! Usage:
//!   cargo run --example device_monitor

use hdc_rs::HdcClient;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== HDC Device Monitor ===\n");

    // Connect to HDC server
    let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    println!("✓ Connected to HDC server\n");

    // Example 1: Wait for any device
    println!("Example 1: Waiting for any device to connect...");
    println!("(If no device is connected, this will block until one connects)\n");

    // Check if devices are already connected
    let initial_devices = client.list_targets().await?;
    if initial_devices.is_empty() {
        println!("No devices currently connected. Waiting...");
        let device = client.wait_for_device().await?;
        println!("✓ Device connected: {}\n", device);
    } else {
        println!("✓ Devices already connected:");
        for device in &initial_devices {
            println!("  - {}", device);
        }
        println!();
    }

    // Example 2: Monitor device list changes
    println!("Example 2: Monitoring device list changes...");
    println!("(Connect or disconnect devices to see updates)");
    println!("Press Ctrl+C to stop\n");

    // Track connection/disconnection events
    let mut previous_set: std::collections::HashSet<String> =
        initial_devices.iter().cloned().collect();
    client
        .monitor_devices(Duration::from_secs(2), |devices| {
            let current_set: std::collections::HashSet<String> = devices.iter().cloned().collect();

            // Find newly connected devices
            let connected: Vec<_> = current_set.difference(&previous_set).cloned().collect();

            // Find disconnected devices
            let disconnected: Vec<_> = previous_set.difference(&current_set).cloned().collect();

            // Update previous set
            previous_set = current_set;

            // Print changes
            if !connected.is_empty() {
                println!("🟢 Device(s) connected:");
                for device in &connected {
                    println!("   + {}", device);
                }
            }

            if !disconnected.is_empty() {
                println!("🔴 Device(s) disconnected:");
                for device in &disconnected {
                    println!("   - {}", device);
                }
            }

            if !connected.is_empty() || !disconnected.is_empty() {
                println!("\n📱 Current devices ({}):", devices.len());
                if devices.is_empty() {
                    println!("   (none)");
                } else {
                    for device in devices {
                        println!("   • {}", device);
                    }
                }
                println!();
            }

            true // Continue monitoring
        })
        .await?;

    Ok(())
}
