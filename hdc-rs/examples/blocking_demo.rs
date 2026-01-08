//! Example: Using the blocking API
//!
//! This demonstrates how to use the blocking (synchronous) API,
//! which is ideal for FFI bindings like PyO3 or synchronous contexts.
//!
//! **Note**: HDC server may close the connection after each command when
//! connected to a specific device. This example demonstrates the recommended
//! pattern of reconnecting to the device before each operation to ensure
//! reliability.

use hdc_rs::blocking::HdcClient;
use hdc_rs::forward::ForwardNode;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();

    println!("=== HDC Blocking API Demo ===\n");

    // Connect to HDC server (synchronous!)
    println!("Connecting to HDC server...");
    let mut client = HdcClient::connect("127.0.0.1:8710")?;
    println!("✓ Connected\n");

    // List devices
    println!("Listing devices...");
    let devices = client.list_targets()?;

    if devices.is_empty() {
        println!("No devices connected. Waiting for device...");
        // Wait for a device
        match client.wait_for_device() {
            Ok(device) => {
                println!("✓ Device found: {}\n", device);
            }
            Err(_) => {
                println!("✗ No device found within timeout");
                return Ok(());
            }
        }
    } else {
        println!("✓ Found {} device(s):", devices.len());
        for (i, device) in devices.iter().enumerate() {
            println!("  {}. {}", i + 1, device);
        }
        println!();

        // Connect to first device
        let device_id = &devices[0];
        println!("Connecting to device: {}", device_id);
        client.connect_device(device_id)?;
        println!("✓ Connected to device\n");

        // Execute shell command
        println!("Executing shell command: uname -a");
        let output = client.shell("uname -a")?;
        println!("Output:\n{}\n", output.trim());

        // Reconnect to device before next command (HDC server may close connection)
        println!("Getting system property: ro.product.model");
        client.connect_device(device_id)?;
        let output = client.shell("param get ro.product.model")?;
        println!("Device model: {}\n", output.trim());

        // Reconnect before port forwarding
        println!("Setting up port forwarding (local:8080 -> device:8080)");
        client.connect_device(device_id)?;
        let local = ForwardNode::Tcp(8080);
        let remote = ForwardNode::Tcp(8080);
        let task_str = format!(
            "{} {}",
            local.as_protocol_string(),
            remote.as_protocol_string()
        );
        match client.fport(local, remote) {
            Ok(result) => {
                println!("✓ Forward created: {}\n", result);

                // Reconnect and clean up
                println!("Removing port forward...");
                client.connect_device(device_id)?;
                client.fport_remove(&task_str)?;
                println!("✓ Forward removed\n");
            }
            Err(e) => {
                println!("✗ Forward failed: {}\n", e);
            }
        }

        // Reconnect for hilog
        println!("Getting device logs (hilog)...");
        client.connect_device(device_id)?;
        let logs = client.hilog(None)?;
        let lines: Vec<&str> = logs.lines().take(10).collect();
        println!("First 10 log lines:");
        for line in lines {
            println!("  {}", line);
        }
        println!();

        // Interactive shell
        println!("\n=== Interactive Shell Mode ===");
        println!("Type 'exit' to quit, 'help' for available commands\n");

        loop {
            print!("hdc $ ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            match input {
                "exit" | "quit" => {
                    println!("Goodbye!");
                    break;
                }
                "help" => {
                    println!("Available commands:");
                    println!("  exit/quit    - Exit interactive shell");
                    println!("  help         - Show this help");
                    println!("  devices      - List connected devices");
                    println!("  clear        - Clear screen");
                    println!("  <command>    - Execute shell command on device");
                }
                "devices" => {
                    let devices = client.list_targets()?;
                    println!("Connected devices:");
                    for device in devices {
                        println!("  - {}", device);
                    }
                }
                "clear" => {
                    print!("\x1B[2J\x1B[1;1H");
                }
                cmd => {
                    // Reconnect before each shell command
                    if let Err(e) = client.connect_device(device_id) {
                        eprintln!("Error reconnecting: {}", e);
                        continue;
                    }
                    match client.shell(cmd) {
                        Ok(output) => {
                            if !output.trim().is_empty() {
                                print!("{}", output);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
            }
        }
    }

    println!("\n✓ Demo completed successfully!");
    Ok(())
}
