//! Simple example demonstrating device monitoring
//!
//! Run with:
//! ```bash
//! cargo run --example device_monitor_demo
//! ```

use hdc_rs::blocking::HdcClient;
use std::collections::HashSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== HDC Device Monitor Demo ===\n");
    println!("This example monitors device connections/disconnections.");
    println!("Try connecting or disconnecting devices to see changes.\n");
    println!("Press Ctrl+C to stop.\n");
    println!("========================================\n");

    let mut client = HdcClient::connect("127.0.0.1:8710")?;

    let mut previous_set: HashSet<String> = HashSet::new();
    let mut change_count = 0;

    // Monitor devices every 2 seconds
    client.monitor_devices(2, |devices| {
        let current_set: HashSet<String> = devices.iter().cloned().collect();

        // Check what changed
        let added: Vec<_> = current_set.difference(&previous_set).collect();
        let removed: Vec<_> = previous_set.difference(&current_set).collect();

        if !added.is_empty() || !removed.is_empty() {
            change_count += 1;
            println!("🔄 Change #{}", change_count);

            if !added.is_empty() {
                println!("  ➕ Devices connected:");
                for device in &added {
                    println!("     - {}", device);
                }
            }

            if !removed.is_empty() {
                println!("  ➖ Devices disconnected:");
                for device in &removed {
                    println!("     - {}", device);
                }
            }

            println!("\n  📱 Current devices ({}): ", devices.len());
            if devices.is_empty() {
                println!("     (none)");
            } else {
                for device in devices {
                    println!("     ✓ {}", device);
                }
            }
            println!();
        }

        previous_set = current_set;

        // Continue monitoring (return true to continue, false to stop)
        true
    })?;

    println!("========================================");
    println!(
        "\n✓ Device monitoring stopped. Detected {} changes.",
        change_count
    );

    Ok(())
}
