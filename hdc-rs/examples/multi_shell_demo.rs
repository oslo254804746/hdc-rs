//! Example: Execute multiple shell commands on the same connection
//!
//! This demonstrates that a single connection can be reused for multiple commands.

use hdc_rs::HdcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to see debug info
    tracing_subscriber::fmt()
        .with_env_filter("hdc_rs=debug")
        .init();

    println!("HDC Rust Client - Multiple Shell Commands Demo");
    println!("===============================================\n");

    // Connect to HDC server
    println!("Connecting to HDC server at 127.0.0.1:8710...");
    let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    println!("✓ Connected\n");

    // List devices
    println!("Finding devices...");
    let devices = client.list_targets().await?;

    if devices.is_empty() {
        eprintln!("✗ No devices found!");
        return Ok(());
    }

    println!("✓ Found {} device(s)\n", devices.len());
    let device = &devices[0];
    println!("Using device: {}\n", device);

    // Connect to device ONCE
    println!("Connecting to device...");
    client.connect_device(device).await?;
    println!("✓ Device connected\n");

    // Now execute MULTIPLE shell commands on the SAME connection
    println!("=== Executing Multiple Shell Commands ===\n");

    let commands = vec![
        ("Get device info", "param get const.product.name"),
        ("Check date", "date"),
        ("List /data", "ls -l /data/local/tmp | head -5"),
        ("Get uptime", "uptime"),
        ("Get hostname", "hostname"),
        ("Check disk space", "df -h /data | head -2"),
        ("List processes", "ps | head -5"),
        ("Get kernel version", "uname -a"),
        ("Check memory", "free -m"),
        ("Get environment", "env | head -5"),
    ];

    for (i, (desc, cmd)) in commands.iter().enumerate() {
        println!("{}. {} (cmd: {})", i + 1, desc, cmd);
        match client.shell(cmd).await {
            Ok(output) => {
                let trimmed = output.trim();
                if trimmed.is_empty() {
                    println!("   (no output)");
                } else {
                    // Print first few lines
                    for line in trimmed.lines().take(3) {
                        println!("   {}", line);
                    }
                    if trimmed.lines().count() > 3 {
                        println!("   ...");
                    }
                }
            }
            Err(e) => println!("   ✗ Error: {}", e),
        }
        println!();
    }

    println!("=== All commands executed successfully! ===");
    println!("Note: All commands used the SAME connection\n");

    // Close connection
    println!("Closing connection...");
    client.close().await?;
    println!("✓ Done");

    Ok(())
}
