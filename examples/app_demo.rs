//! Example: App installation and management
//!
//! This example demonstrates how to install and uninstall apps.

use hdc_rs::{HdcClient, InstallOptions, UninstallOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Connect to HDC server
    let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    println!("✓ Connected to HDC server");

    // List available devices
    let devices = client.list_targets().await?;
    if devices.is_empty() {
        println!("✗ No devices found");
        return Ok(());
    }
    println!("✓ Found {} device(s)", devices.len());

    // Select first device
    client.connect_device(&devices[0]).await?;
    println!("✓ Connected to device: {}", devices[0]);

    // Example 1: Install an app (replace if exists)
    println!("\n=== Example 1: Install App (Replace) ===");
    let app_path = "path/to/your/app.hap";
    println!("Installing app from: {}", app_path);
    println!("Note: Replace with actual .hap file path");

    let install_opts = InstallOptions::new().replace(true);
    match client.install(&[app_path], install_opts).await {
        Ok(response) => println!("✓ Install result:\n{}", response),
        Err(e) => println!("✗ Install failed: {}", e),
    }

    // Example 2: Install multiple packages
    println!("\n=== Example 2: Install Multiple Packages ===");
    let packages = vec!["app1.hap", "app2.hap"];
    println!("Installing packages: {:?}", packages);
    println!("Note: Replace with actual .hap file paths");

    let install_opts = InstallOptions::new();
    match client.install(&packages.to_vec(), install_opts).await {
        Ok(response) => println!("✓ Install result:\n{}", response),
        Err(e) => println!("✗ Install failed: {}", e),
    }

    // Example 3: Install shared bundle
    println!("\n=== Example 3: Install Shared Bundle ===");
    let shared_bundle = "shared_bundle.hsp";
    println!("Installing shared bundle: {}", shared_bundle);

    let install_opts = InstallOptions::new().shared(true);
    match client.install(&[shared_bundle], install_opts).await {
        Ok(response) => println!("✓ Install result:\n{}", response),
        Err(e) => println!("✗ Install failed: {}", e),
    }

    // Example 4: Uninstall an app
    println!("\n=== Example 4: Uninstall App ===");
    let package_name = "com.example.app";
    println!("Uninstalling package: {}", package_name);

    let uninstall_opts = UninstallOptions::new();
    match client.uninstall(package_name, uninstall_opts).await {
        Ok(response) => println!("✓ Uninstall result:\n{}", response),
        Err(e) => println!("✗ Uninstall failed: {}", e),
    }

    // Example 5: Uninstall but keep data
    println!("\n=== Example 5: Uninstall (Keep Data) ===");
    let package_name = "com.example.app";
    println!("Uninstalling package (keeping data): {}", package_name);

    let uninstall_opts = UninstallOptions::new().keep_data(true);
    match client.uninstall(package_name, uninstall_opts).await {
        Ok(response) => println!("✓ Uninstall result:\n{}", response),
        Err(e) => println!("✗ Uninstall failed: {}", e),
    }

    // Example 6: Uninstall shared bundle
    println!("\n=== Example 6: Uninstall Shared Bundle ===");
    let shared_package = "com.example.shared";
    println!("Uninstalling shared package: {}", shared_package);

    let uninstall_opts = UninstallOptions::new().shared(true);
    match client.uninstall(shared_package, uninstall_opts).await {
        Ok(response) => println!("✓ Uninstall result:\n{}", response),
        Err(e) => println!("✗ Uninstall failed: {}", e),
    }

    println!("\n✓ Examples completed");
    Ok(())
}
