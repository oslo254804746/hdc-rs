//! Example demonstrating blocking hilog_stream and monitor_devices
//!
//! This example shows how to use the blocking API to:
//! 1. Monitor device connections/disconnections
//! 2. Stream device logs in real-time
//!
//! Run with:
//! ```bash
//! cargo run --example blocking_monitor
//! ```

use hdc_rs::blocking::HdcClient;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Blocking HDC Monitor Demo ===\n");
    println!("Choose an example:");
    println!("  1. Monitor device changes");
    println!("  2. Stream device logs");
    println!("  3. Combined example");
    println!();

    // For demo purposes, run example 1 (monitor devices)
    // Change to example 2 or 3 to test other features
    let example = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);

    match example {
        1 => monitor_devices_example()?,
        2 => hilog_stream_example()?,
        3 => combined_example()?,
        _ => {
            println!("Invalid example number. Use: cargo run --example blocking_monitor <1|2|3>");
        }
    }

    Ok(())
}

fn monitor_devices_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Monitoring device changes for 30 seconds...\n");

    let mut client = HdcClient::connect("127.0.0.1:8710")?;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let start_time = SystemTime::now();

    // Start a thread to stop after 30 seconds
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(30));
        println!("\n⏰ 30 seconds elapsed, stopping monitoring...");

        r.store(false, Ordering::SeqCst);
    });

    // Monitor devices every 2 seconds
    client.monitor_devices(2, |devices| {
        let elapsed = SystemTime::now()
            .duration_since(start_time)
            .unwrap_or_default();

        println!("📱 [{}s] Devices changed:", elapsed.as_secs());
        if devices.is_empty() {
            println!("  ⚠️  No devices connected");
        } else {
            for device in devices {
                println!("  ✓ {}", device);
            }
        }
        println!();

        // Continue monitoring while running flag is true
        running.load(Ordering::SeqCst)
    })?;

    println!("Device monitoring stopped.");
    Ok(())
}

fn hilog_stream_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Streaming device logs for 20 seconds...\n");

    let mut client = HdcClient::connect("127.0.0.1:8710")?;

    // Get first available device
    let devices = client.list_targets()?;
    if devices.is_empty() {
        println!("⚠️  No devices found. Please connect a device first.");
        return Ok(());
    }

    let device_id = &devices[0];
    println!("Using device: {}\n", device_id);
    client.connect_device(device_id)?;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Start a thread to stop after 20 seconds
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(20));
        println!("\n⏰ 20 seconds elapsed, stopping log stream...");
        r.store(false, Ordering::SeqCst);
    });

    println!("📋 Streaming logs...\n");
    println!("----------------------------------------");

    let mut line_count = 0;

    // Stream logs with optional filter
    // Example: Stream all logs
    client.hilog_stream(None, |log_chunk| {
        print!("{}", log_chunk);
        line_count += log_chunk.lines().count();
        running.load(Ordering::SeqCst)
    })?;

    // Example: Stream with tag filter
    // client.hilog_stream(Some("-t MyTag -v debug"), |log_chunk| {
    //     print!("{}", log_chunk);
    //     running.load(Ordering::SeqCst)
    // })?;

    println!("----------------------------------------");
    println!("\n✓ Log streaming stopped. Received {} lines.", line_count);
    Ok(())
}

fn combined_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Combined monitoring and logging\n");

    let running = Arc::new(AtomicBool::new(true));
    let r1 = running.clone();
    let r2 = running.clone();

    // Start device monitoring in a separate thread
    let monitor_thread = thread::spawn(
        move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let mut client = HdcClient::connect("127.0.0.1:8710")?;
            let mut check_count = 0;

            client.monitor_devices(3, move |devices| {
                check_count += 1;
                println!(
                    "🔄 [Monitor Check #{}] Devices: {} connected",
                    check_count,
                    devices.len()
                );
                for device in devices {
                    println!("    - {}", device);
                }

                // Stop after 5 checks (15 seconds) or if global stop flag is set
                if check_count >= 5 {
                    println!("📊 [Monitor] Completed 5 checks, stopping...");
                    r1.store(false, Ordering::SeqCst);
                    false
                } else {
                    r1.load(Ordering::SeqCst)
                }
            })?;

            Ok(())
        },
    );

    // Wait a bit then start log streaming
    thread::sleep(Duration::from_secs(2));

    let log_thread = thread::spawn(
        move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let mut client = HdcClient::connect("127.0.0.1:8710")?;

            let devices = client.list_targets()?;
            if !devices.is_empty() {
                client.connect_device(&devices[0])?;
                println!("📋 [Logger] Connected to device: {}\n", devices[0]);

                let mut line_count = 0;
                client.hilog_stream(None, |log_chunk| {
                    if !log_chunk.is_empty() {
                        let chunk_lines = log_chunk.lines().count();
                        line_count += chunk_lines;

                        // Only print first 10 lines to avoid cluttering output
                        if line_count <= 10 {
                            print!("📄 [Log] {}", log_chunk);
                        } else if line_count == 11 {
                            println!("📄 [Log] ... (suppressing further output) ...");
                        }

                        // Stop after 30 lines or if global stop flag is set
                        if line_count >= 30 {
                            println!("\n📊 [Logger] Received {} lines, stopping...", line_count);
                            r2.store(false, Ordering::SeqCst);
                            false
                        } else {
                            r2.load(Ordering::SeqCst)
                        }
                    } else {
                        r2.load(Ordering::SeqCst)
                    }
                })?;
            } else {
                println!("⚠️  [Logger] No devices found");
            }

            Ok(())
        },
    );

    // Wait for both threads to complete
    println!("⏳ Running combined example...\n");

    let monitor_result = monitor_thread.join().unwrap();
    let log_result = log_thread.join().unwrap();

    if let Err(e) = monitor_result {
        eprintln!("❌ Monitor thread error: {}", e);
    }
    if let Err(e) = log_result {
        eprintln!("❌ Logger thread error: {}", e);
    }

    println!("\n✓ Combined example completed");
    Ok(())
}
