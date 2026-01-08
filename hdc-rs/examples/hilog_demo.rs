//! HDC Hilog Example
//!
//! This example demonstrates how to use the hilog functionality to read device logs.
//!
//! Usage:
//!   cargo run --example hilog_demo [device_id] [args]
//!
//! Examples:
//!   cargo run --example hilog_demo                    # List devices
//!   cargo run --example hilog_demo ABC123            # Show all logs
//!   cargo run --example hilog_demo ABC123 "-t app"   # Show only app logs
//!   cargo run --example hilog_demo ABC123 "-h"       # Show hilog help

use hdc_rs::HdcClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args: Vec<String> = env::args().collect();

    // Connect to HDC server
    let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    println!("✓ Connected to HDC server");

    // List available devices
    let devices = client.list_targets().await?;
    if devices.is_empty() {
        println!("No devices found. Please connect a device first.");
        return Ok(());
    }

    println!("\nAvailable devices:");
    for device in &devices {
        println!("  - {}", device);
    }

    // Get device ID from arguments or use first device
    let device_id = if args.len() > 1 {
        args[1].clone()
    } else {
        println!("\nUsage: {} [device_id] [hilog_args]", args[0]);
        println!("Example: {} {} \"-t app\"", args[0], devices[0]);
        return Ok(());
    };

    // Connect to the specified device
    println!("\nConnecting to device: {}", device_id);
    client.connect_device(&device_id).await?;
    println!("✓ Connected to device");

    // Get hilog arguments if provided
    let hilog_args = if args.len() > 2 {
        Some(args[2..].join(" "))
    } else {
        None
    };

    println!("\n========== Device Logs ==========");

    // Example 1: Get logs as a string (buffered, stops after timeout)
    if let Some(ref args) = hilog_args {
        println!("Running: hilog {}", args);
        match client.hilog(Some(args)).await {
            Ok(logs) => {
                println!("{}", logs);
            }
            Err(e) => {
                eprintln!("Error reading logs: {}", e);
            }
        }
    } else {
        // Example 2: Stream logs continuously with callback
        println!("Streaming logs (Ctrl+C to stop)...\n");

        let result = client
            .hilog_stream(None, |log_chunk| {
                print!("{}", log_chunk);
                true // Continue streaming
            })
            .await;

        if let Err(e) = result {
            eprintln!("\nError streaming logs: {}", e);
        }
    }

    Ok(())
}
