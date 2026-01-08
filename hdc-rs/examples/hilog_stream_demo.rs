//! Simple example demonstrating hilog_stream
//!
//! Run with:
//! ```bash
//! cargo run --example hilog_stream_demo
//! ```

use hdc_rs::blocking::HdcClient;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== HDC Hilog Stream Demo ===\n");

    let mut client = HdcClient::connect("127.0.0.1:8710")?;

    // Get available devices
    let devices = client.list_targets()?;
    if devices.is_empty() {
        eprintln!("❌ No devices found. Please connect a device.");
        return Ok(());
    }

    println!("📱 Available devices:");
    for (i, device) in devices.iter().enumerate() {
        println!("  {}. {}", i + 1, device);
    }

    // Connect to first device
    let device_id = &devices[0];
    println!("\n✓ Connecting to: {}", device_id);
    client.connect_device(device_id)?;

    println!("\n📋 Starting log stream...");
    println!("   (Press Ctrl+C to stop)\n");
    println!("========================================");

    let mut line_count = 0;

    // Stream logs - will run until interrupted or connection closes
    let result = client.hilog_stream(None, |log_chunk| {
        // Print the log chunk
        print!("{}", log_chunk);
        io::stdout().flush().unwrap();

        line_count += log_chunk.lines().count();

        // Continue streaming (return true)
        // You could add logic here to stop based on conditions:
        // - return false to stop
        // - check line_count, time, or other conditions
        true
    });

    println!("========================================");
    match result {
        Ok(_) => println!(
            "\n✓ Log stream ended normally. Received {} lines.",
            line_count
        ),
        Err(e) => println!("\n❌ Log stream error: {}", e),
    }

    Ok(())
}
