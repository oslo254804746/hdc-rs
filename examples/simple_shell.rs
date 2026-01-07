//! Example: Execute shell commands on a device

use hdc_rs::HdcClient;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("hdc_rs=info")
        .init();

    println!("HDC Rust Client - Simple Shell Example");
    println!("========================================\n");

    // Connect to HDC server
    println!("Connecting to HDC server at 127.0.0.1:8710...");
    let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    println!("✓ Connected\n");

    // List devices
    println!("Finding devices...");
    let devices = client.list_targets().await?;

    if devices.is_empty() {
        eprintln!("✗ No devices found!");
        eprintln!("Please connect a device and try again.");
        return Ok(());
    }

    // Select device (use first one if only one device)
    let device = if devices.len() == 1 {
        println!("✓ Using device: {}\n", devices[0]);
        &devices[0]
    } else {
        println!("Multiple devices found:");
        for (i, dev) in devices.iter().enumerate() {
            println!("  [{}] {}", i + 1, dev);
        }
        print!("\nSelect device (1-{}): ", devices.len());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let index: usize = input.trim().parse()?;

        if index < 1 || index > devices.len() {
            eprintln!("Invalid selection!");
            return Ok(());
        }

        &devices[index - 1]
    };

    // Connect to the selected device
    println!("\nConnecting to device: {}", device);
    client.connect_device(device).await?;
    println!("✓ Connected to device\n");

    // Execute some example commands
    println!("\n--- Executing Shell Commands ---\n");

    // Command 1: Get device info
    println!("1. Getting device properties:");
    match client.shell("getprop ro.product.model").await {
        Ok(output) => println!("   Device model: {}", output.trim()),
        Err(e) => println!("   Error: {}", e),
    }

    // Command 2: List directory
    println!("\n2. Listing /data directory:");
    match client.shell("ls -la /data").await {
        Ok(output) => {
            for line in output.lines().take(10) {
                println!("   {}", line);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    // Command 3: Get system info
    println!("\n3. System uptime:");
    match client.shell("uptime").await {
        Ok(output) => println!("   {}", output.trim()),
        Err(e) => println!("   Error: {}", e),
    }

    // Interactive shell mode
    println!("\n--- Interactive Shell Mode ---");
    println!("Type 'exit' or 'quit' to exit\n");

    loop {
        print!("hdc# ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let cmd = input.trim();

        if cmd.is_empty() {
            continue;
        }

        if cmd == "exit" || cmd == "quit" {
            break;
        }

        match client.shell(cmd).await {
            Ok(output) => print!("{}", output),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    println!("\nClosing connection...");
    client.close().await?;
    println!("✓ Done");

    Ok(())
}
