//! File Transfer Example
//!
//! This example demonstrates how to send and receive files between local and device.
//!
//! Usage:
//!   cargo run --example file_demo [device_id]

use hdc_rs::{FileTransferOptions, HdcClient};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== HDC File Transfer Example ===\n");

    let args: Vec<String> = env::args().collect();

    // Connect to HDC server
    let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    println!("✓ Connected to HDC server");

    // List available devices
    let devices = client.list_targets().await?;
    if devices.is_empty() {
        println!("✗ No devices found. Please connect a device first.");
        return Ok(());
    }

    println!("Available devices:");
    for device in &devices {
        println!("  - {}", device);
    }

    // Get device ID from arguments or use first device
    let device_id = if args.len() > 1 {
        args[1].clone()
    } else {
        devices[0].clone()
    };

    // Connect to the specified device
    println!("\n🔗 Connecting to device: {}", device_id);
    client.connect_device(&device_id).await?;
    println!("✓ Connected to device\n");

    // Example 1: Send a file with options
    println!("📤 Example 1: Send file to device");
    println!("Note: This is a demonstration. Update paths with actual files.\n");

    println!("Command examples:");
    println!("  1. Send file with default options:");
    println!("     client.file_send(\"local.txt\", \"/data/local/tmp/remote.txt\", FileTransferOptions::new()).await?;");

    println!("\n  2. Send file preserving timestamp:");
    println!("     let opts = FileTransferOptions::new().hold_timestamp(true);");
    println!("     client.file_send(\"local.txt\", \"/data/local/tmp/remote.txt\", opts).await?;");

    println!("\n  3. Send file with compression:");
    println!("     let opts = FileTransferOptions::new().compress(true);");
    println!("     client.file_send(\"local.txt\", \"/data/local/tmp/remote.txt\", opts).await?;");

    println!("\n  4. Send file with sync mode (only if newer):");
    println!("     let opts = FileTransferOptions::new().sync_mode(true);");
    println!("     client.file_send(\"local.txt\", \"/data/local/tmp/remote.txt\", opts).await?;");

    println!("\n  5. Send to debug application directory:");
    println!("     let opts = FileTransferOptions::new().debug_dir(true);");
    println!("     client.file_send(\"local.txt\", \"remote.txt\", opts).await?;");

    // Example 2: Receive a file
    println!("\n📥 Example 2: Receive file from device");
    println!("Command examples:");
    println!("  1. Receive file with default options:");
    println!("     client.file_recv(\"/data/local/tmp/remote.txt\", \"local.txt\", FileTransferOptions::new()).await?;");

    println!("\n  2. Receive file preserving timestamp:");
    println!("     let opts = FileTransferOptions::new().hold_timestamp(true);");
    println!("     client.file_recv(\"/data/local/tmp/remote.txt\", \"local.txt\", opts).await?;");

    println!("\n  3. Receive file with sync mode:");
    println!("     let opts = FileTransferOptions::new().sync_mode(true);");
    println!("     client.file_recv(\"/data/local/tmp/remote.txt\", \"local.txt\", opts).await?;");

    // Interactive demo
    println!("\n🔄 Interactive Demo");
    println!("─────────────────────────────────────");

    // Try to send a test file (if exists)
    println!("\nAttempting to send test file...");
    println!("(Create a file named 'test_upload.txt' in the current directory to test)");

    // Check if test file exists
    let test_file =
        "D:\\private\\ClionProjects\\developtools_hdc\\hdc-rs\\test_upload.txttest_upload.txt";
    if std::path::Path::new(test_file).exists() {
        println!("✓ Found test file: {}", test_file);
        println!("Uploading to device...");

        let opts = FileTransferOptions::new().hold_timestamp(true);
        match client
            .file_send(test_file, "/data/local/tmp/test_upload.txt", opts)
            .await
        {
            Ok(response) => {
                println!("✓ Upload successful!");
                println!("Response: {}", response.trim());
                let resp = client.shell("cat /data/local/tmp/test_upload.txt").await?;
                assert_eq!(&resp, "this is a demo");
            }
            Err(e) => {
                println!("✗ Upload failed: {}", e);
            }
        }
    } else {
        println!("ℹ Test file not found. Skipping upload test.");
    }

    // Try to receive a common file
    println!("\nAttempting to receive a file from device...");
    println!("Trying to download: /system/bin/ls");

    let opts = FileTransferOptions::new();
    match client
        .file_recv("/system/bin/ls", "downloaded_ls", opts)
        .await
    {
        Ok(response) => {
            println!("✓ Download successful!");
            println!("Response: {}", response.trim());
            println!("File saved as: downloaded_ls");
        }
        Err(e) => {
            println!("✗ Download failed: {}", e);
            println!("(This is expected if the file doesn't exist or permissions are denied)");
        }
    }

    println!("\n✓ File transfer example completed!");
    println!("\nTip: Create 'test_upload.txt' and run again to test actual file upload.");

    Ok(())
}
