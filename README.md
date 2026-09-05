# HDC-RS

[![Crates.io](https://img.shields.io/crates/v/hdc-rs.svg)](https://crates.io/crates/hdc-rs)
[![Documentation](https://docs.rs/hdc-rs/badge.svg)](https://docs.rs/hdc-rs)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.71%2B-orange.svg)](https://www.rust-lang.org)
[![Rust Validation](https://github.com/oslo254804746/hdc-rs/actions/workflows/rust-validation.yml/badge.svg?branch=master)](https://github.com/oslo254804746/hdc-rs/actions/workflows/rust-validation.yml)

**Release:** 0.2.0 · **Status:** Beta

A pure Rust implementation of the **HarmonyOS Device Connector (HDC)** client library, providing both async and blocking APIs for interacting with HarmonyOS/OpenHarmony devices.

> **Note:** HDC is to HarmonyOS what ADB is to Android - a bridge for device communication, debugging, and development.

---

## ✨ Features

- 🚀 **Async/await** - Built on Tokio for efficient async I/O
- 🔄 **Blocking API** - Synchronous wrapper for FFI bindings (PyO3, JNI, etc.)
- 📱 **Device Management** - List, connect, and monitor devices
- 💻 **Shell Commands** - Execute commands on devices with full output
- 🔌 **Port Forwarding** - TCP, Unix sockets, JDWP, and Ark debugger support
- 📦 **App Management** - Install/uninstall HAP and HSP packages
- 📁 **File Transfer** - Efficient bidirectional file transfer with compression
- 🔍 **Device Monitoring** - Real-time device connection/disconnection events
- 📋 **Log Streaming** - Continuous or buffered hilog reading
- 🛡️ **Type-safe API** - Rust's type system ensures correctness
- ⚡ **Zero-copy** - Efficient data handling with `bytes` crate
- 🎯 **Error Handling** - Comprehensive error types with context

## 📋 Table of Contents

- [Features](#-features)
- [Installation](#-installation)
- [Quick Start](#-quick-start)
- [Examples](#-examples)
- [API Documentation](#-api-documentation)
- [Architecture](#️-architecture)
- [Protocol Details](#-protocol-details)
- [Python Bindings](#-python-bindings)
- [API Reference](#-api-reference)
- [Development](#-development)
- [Troubleshooting](#-troubleshooting)
- [Performance](#-performance)
- [Roadmap](#️-roadmap)
- [Contributing](#-contributing)
- [License](#-license)
- [Resources](#-resources)

## 🔧 Installation

### Prerequisites

- Rust 1.71 or later
- HDC server must be installed and running
- A HarmonyOS/OpenHarmony device connected via USB or network

### Add to Your Project

Add this to your `Cargo.toml`:

```toml
[dependencies]
hdc-rs = "0.2"
tokio = { version = "1", features = ["full"] }
```

### Feature Flags

- `blocking` - Enable synchronous/blocking API for FFI bindings

```toml
[dependencies]
hdc-rs = { version = "0.2", features = ["blocking"] }
```

## 🚀 Quick Start

### Async API (Recommended)

```rust
use hdc_rs::HdcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to HDC server
    let mut client = HdcClient::connect("127.0.0.1:8710").await?;

    // List connected devices
    let devices = client.list_targets().await?;
    println!("Devices: {:?}", devices);

    if devices.is_empty() {
        println!("No devices connected!");
        return Ok(());
    }

    // Select and connect to first device
    let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    client.connect_device(&devices[0]).await?;
    println!("Connected to device: {}", devices[0]);

    // Execute shell command on the selected device
    let output = client.shell("ls -l /data").await?;
    println!("Output:\n{}", output);

    Ok(())
}
```

### Blocking API (for FFI/PyO3)

Enable the `blocking` feature for synchronous API:

```toml
[dependencies]
hdc-rs = { version = "0.2", features = ["blocking"] }
```

```rust
use hdc_rs::blocking::HdcClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to HDC server (synchronous!)
    let mut client = HdcClient::connect("127.0.0.1:8710")?;

    // List connected devices
    let devices = client.list_targets()?;
    println!("Devices: {:?}", devices);

    if !devices.is_empty() {
        // Connect and execute command
        let mut client = HdcClient::connect("127.0.0.1:8710")?;
        client.connect_device(&devices[0])?;
        let output = client.shell("uname -a")?;
        println!("Output: {}", output);
    }

    Ok(())
}
```

## 📚 Examples

The repository includes several examples demonstrating different features:

| Example | Description | Command |
|---------|-------------|---------|
| `list_devices` | List all connected devices | `cargo run --example list_devices` |
| `simple_shell` | Interactive shell session | `cargo run --example simple_shell` |
| `blocking_demo` | Synchronous API demo | `cargo run --example blocking_demo --features blocking` |
| `device_monitor` | Monitor device connections | `cargo run --example device_monitor` |
| `file_demo` | File transfer operations | `cargo run --example file_demo` |
| `forward_demo` | Port forwarding setup | `cargo run --example forward_demo` |
| `app_demo` | App install/uninstall | `cargo run --example app_demo` |
| `hilog_demo` | Device log streaming | `cargo run --example hilog_demo` |
| `comprehensive` | All features combined | `cargo run --example comprehensive` |

### Running Examples

```bash
# List all connected devices
cargo run --example list_devices

# Blocking API demo (synchronous)
cargo run --example blocking_demo --features blocking

# Monitor device connections/disconnections
cargo run --example device_monitor

# Interactive shell
cargo run --example simple_shell

# File transfer demo
cargo run --example file_demo

# Port forwarding demo
cargo run --example forward_demo

# App installation demo
cargo run --example app_demo

# Hilog (device logs) demo
cargo run --example hilog_demo

# Comprehensive example (all features)
cargo run --example comprehensive

# Enable debug logging for troubleshooting
RUST_LOG=hdc_rs=debug cargo run --example list_devices
```

## 📖 API Documentation

For detailed API documentation, visit [docs.rs/hdc-rs](https://docs.rs/hdc-rs).

### Core Types

- **`HdcClient`** - Main async client for HDC communication
- **`blocking::HdcClient`** - Synchronous wrapper for FFI bindings
- **`HdcError`** - Comprehensive error type with context
- **`ForwardNode`** - Port forwarding endpoint specification
- **`InstallOptions`** / **`UninstallOptions`** - App management options
- **`FileTransferOptions`** - File transfer configuration
- **`TargetBootMode`** / **`TargetMode`** - Device control command options

### Client SDK Examples

Connect or disconnect a TCP/manual target:

```rust
use hdc_rs::HdcClient;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.target_connect("192.168.0.2:10178").await?;

let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.target_disconnect("192.168.0.2:10178").await?;
# Ok(())
# }
```

Run common device-control commands:

```rust
use hdc_rs::{HdcClient, TargetBootMode, TargetMode};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.target_mount().await?;

let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.target_boot(Some(TargetBootMode::Recovery)).await?;

let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.smode(true).await?;

let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.tmode(TargetMode::Port(Some(10178))).await?;
# Ok(())
# }
```

List debug process identifiers:

```rust
use hdc_rs::HdcClient;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
let jpids = client.jpid().await?;
println!("{:?}", jpids);
# Ok(())
# }
```

Use extended file and app options:

```rust
use hdc_rs::{FileTransferOptions, HdcClient, InstallOptions, UninstallOptions};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
let file_opts = FileTransferOptions::new()
    .compress(true)
    .debug_dir(true)
    .cwd("/data/local/tmp");
client.file_send("local.txt", "remote.txt", file_opts).await?;

let mut client = HdcClient::connect("127.0.0.1:8710").await?;
let install_opts = InstallOptions::new()
    .replace(true)
    .cwd("/tmp")
    .wait_time(30)
    .grant_permissions(true);
client.install(&["app.hap"], install_opts).await?;

let mut client = HdcClient::connect("127.0.0.1:8710").await?;
let uninstall_opts = UninstallOptions::new()
    .keep_data(true)
    .user_id("100");
client.uninstall("com.example.app", uninstall_opts).await?;
# Ok(())
# }
```

See [docs/hdc-client-api-matrix.md](docs/hdc-client-api-matrix.md) for the async, blocking, and Python API coverage matrix.

The matrix distinguishes implemented API surface from device validation. Rows
that require a device are covered only by ignored integration tests and need a
running HDC server plus an authorized HarmonyOS/OpenHarmony device; CI does not
claim real-device validation for those rows.

## 🏗️ Architecture

### System Overview

```
┌─────────────────┐
│   Your App      │  ◄── Your Rust application
└────────┬────────┘
         │ (uses hdc-rs API)
         ▼
┌─────────────────┐
│    hdc-rs       │  ◄── This library (Rust)
│  ┌───────────┐  │
│  │  Client   │  │  - Connection management
│  │  Protocol │  │  - Packet codec
│  │  Commands │  │  - Error handling
│  └───────────┘  │
└────────┬────────┘
         │ (TCP socket: 127.0.0.1:8710)
         ▼
┌─────────────────┐
│  HDC Server     │  ◄── Native HDC daemon
│  (daemon)       │
└────────┬────────┘
         │ (USB/TCP connection)
         ▼
┌─────────────────┐
│ HarmonyOS       │  ◄── Target device
│ Device          │
└─────────────────┘
```

### Project Structure

```
hdc-rs/
├── hdc-rs/              # Core Rust library
│   ├── src/
│   │   ├── lib.rs       # Public API
│   │   ├── client.rs    # HdcClient implementation
│   │   ├── blocking.rs  # Synchronous wrapper
│   │   ├── error.rs     # Error types
│   │   ├── app.rs       # App management
│   │   ├── file.rs      # File transfer
│   │   ├── forward.rs   # Port forwarding
│   │   └── protocol/    # Protocol implementation
│   │       ├── packet.rs    # Packet codec
│   │       ├── command.rs   # Command builders
│   │       └── channel.rs   # Channel management
│   └── Cargo.toml
├── hdc-rs-py/           # Python bindings (PyO3)
│   ├── src/lib.rs
│   └── Cargo.toml
├── examples/            # Usage examples
└── tests/               # Integration tests
```

## 🔌 Protocol Details

### Packet Format

HDC uses a simple length-prefixed binary protocol over TCP:

```
┌──────────────────────────────┐
│  4 bytes: Payload Length     │  (Big-endian u32)
├──────────────────────────────┤
│  N bytes: Payload Data       │  (Command + Arguments)
└──────────────────────────────┘
```

### Connection Lifecycle

```
Client                           Server
  │                                │
  ├──── TCP Connect ──────────────>│
  │                                │
  │<──── Handshake (Channel ID) ───┤
  │                                │
  ├──── Connect Key ──────────────>│
  │                                │
  │<──── OK ────────────────────────┤
  │                                │
  ├──── Command ──────────────────>│
  │                                │
  │<──── Response ──────────────────┤
  │                                │
  ├──── Close ────────────────────>│
  │                                │
```

### Supported Commands

| Command | Description | Status |
|---------|-------------|--------|
| `list targets` | List connected devices | ✅ Implemented |
| `checkserver` | Get server version | ✅ Implemented |
| `tmode port <port>` | Connect device over TCP | ✅ Implemented |
| `shell <cmd>` | Execute shell command | ✅ Implemented |
| `file send <local> <remote>` | Upload file | ✅ Implemented |
| `file recv <remote> <local>` | Download file | ✅ Implemented |
| `fport <local> <remote>` | Forward port | ✅ Implemented |
| `rport <remote> <local>` | Reverse forward | ✅ Implemented |
| `install <path>` | Install app | ✅ Implemented |
| `uninstall <pkg>` | Uninstall app | ✅ Implemented |
| `hilog` | Stream device logs | ✅ Implemented |
| `wait-for-device` | Wait for device | ✅ Implemented |

“Implemented” means the client API is exposed; it is separate from real-device
validation. Device-dependent integration tests are ignored by default and
require a running HDC server plus an authorized HarmonyOS/OpenHarmony device.
CI does not claim hardware validation for these commands.

## 🐍 Python Bindings

This project includes Python bindings built with PyO3. See [`hdc-rs-py/README.md`](hdc-rs-py/README.md) for complete documentation.

### Quick Example

```python
from hdc_rs_py import HdcClient

# Connect and list devices
client = HdcClient("127.0.0.1:8710")
devices = client.list_targets()
print(f"Devices: {devices}")

if devices:
    # Execute command
    client = HdcClient("127.0.0.1:8710")
    client.connect_device(devices[0])
    output = client.shell("uname -a")
    print(output)
```

### Installation

```bash
cd hdc-rs-py
pip install maturin
maturin develop  # Development mode
# or
maturin build --release  # Build wheel
pip install target/wheels/hdc_rs_py-*.whl
```

## 🔍 API Reference

### HdcClient

Main client for HDC communication.

#### Connection Methods

- `connect(address)` - Connect to HDC server
- `close()` - Close connection
- `is_connected()` - Check if connected

#### Device Management

- `list_targets()` - List all connected devices
- `connect_device(device_id)` - Select a device for subsequent commands
- `check_server()` - Get server version
- `wait_for_device()` - Block until a device is connected
- `monitor_devices(interval, callback)` - Monitor device list changes with polling
  - `interval`: Polling interval (e.g., `Duration::from_secs(2)`)
  - `callback`: Function called when device list changes, return `false` to stop

The one-shot local tasks `list_targets()`, `list_targets_verbose()`,
`check_server()`, `version()`, `help()`, `discover()`, and `check_device()` drain
their channel and disconnect before returning. Create a fresh client before a
subsequent terminal task. `wait_for_device()` retains its continuous wait
semantics.

#### Target Control

- `target_connect(key)` / `target_disconnect(key)` - Connect or disconnect a TCP/manual target
- `connect_any()` / `reconnect_target(key)` - Select or reconnect a target
- `target_mount()` - Mount the target filesystem
- `target_boot(mode)` - Boot the target with an optional upstream mode
- `smode(enable_root)` - Switch daemon privilege mode
- `tmode(mode)` - Switch target transport mode

The target-control task methods consume their channel and leave the client
disconnected when they return. Reconnect before issuing another terminal task.

#### Command Execution

- `shell(cmd)` - Execute shell command on the currently selected device
  - **Important**: Must call `connect_device()` first, or server will return error
- `shell_on_device(device_id, cmd)` - Execute shell command on specific device
- `target_command(device_id, cmd)` - Execute any command on specific device

#### Port Forwarding

- `fport(local, remote)` - Forward local traffic to remote device
  - Example: `fport(ForwardNode::Tcp(8080), ForwardNode::Tcp(8081))`
- `rport(remote, local)` - Reverse forward remote traffic to local host
  - Example: `rport(ForwardNode::Tcp(9090), ForwardNode::Tcp(9091))`
- `fport_list()` - List all active forward/reverse tasks
- `fport_remove(task_str)` - Remove a forward task by task string
  - Example: `fport_remove("tcp:8080 tcp:8081")`

Creating an `fport()` or `rport()` mapping drains the terminal task and
disconnects the client on return. Reconnect before issuing another terminal
task; `fport_list()` and `fport_remove()` use their own temporary server
connections.

**Forward Node Types:**
- `ForwardNode::Tcp(port)` - TCP port
- `ForwardNode::LocalFilesystem(path)` - Unix domain socket (filesystem)
- `ForwardNode::LocalReserved(name)` - Unix domain socket (reserved)
- `ForwardNode::LocalAbstract(name)` - Unix domain socket (abstract)
- `ForwardNode::Dev(name)` - Device
- `ForwardNode::Jdwp(pid)` - JDWP (Java Debug Wire Protocol, remote only)
- `ForwardNode::Ark { pid, tid, debugger }` - Ark debugger (remote only)

#### App Management

- `install(paths, options)` - Install application package(s)
  - `paths`: Single or multiple `.hap`/`.hsp` files or directories
  - `options`: `InstallOptions::new().replace(true).cwd("/tmp").wait_time(180).user_id("100").list_options(false).grant_permissions(true)`
    - `replace`: Replace existing application
    - `shared`: Install shared bundle for multi-apps
    - `cwd`: Install working directory
    - `wait_time`: Wait time in seconds (`-w`)
    - `user_id`: Numeric user ID (`-u`)
    - `list_options`: Request install option help (`-h`)
    - `grant_permissions`: Grant permissions after install (`-g`)
- `uninstall(package, options)` - Uninstall application package
  - `package`: Package name passed as the upstream `-n` option (e.g., `"com.example.app"`)
  - `options`: `UninstallOptions::new().keep_data(true).shared(false).module_name("entry").version_code("42").user_id("100").list_options(false)`
    - `keep_data`: Keep the data and cache directories
    - `shared`: Remove shared bundle
    - `module_name`: Module name (`-m`)
    - `version_code`: Numeric version code (`-v`)
    - `user_id`: Numeric user ID (`-u`)
    - `list_options`: Request uninstall option help (`-h`)

Install/uninstall daemon option/value pairs must each be encoded in one quoted
HDC argument (for example, `"-w 180"` or `"-m entry"`). The `-cwd` option
remains two separate arguments and may contain spaces when host-quoted. Numeric
and module option values must be non-empty and safe for the HDC command parser;
they cannot contain whitespace, control characters, quotes, or shell-injection
characters.

#### Log Management

- `hilog(args)` - Read device logs (buffered mode)
  - `args`: Optional hilog arguments (e.g., `"-h"` for help, `"-t app"` for app logs)
  - Returns all logs as a string after timeout
- `hilog_stream(args, callback)` - Streaming hilog to given callback

#### File Transfer

- `file_send(local, remote, options)` - Send file to device
  - `local`: Local file path
  - `remote`: Remote device path
  - `options`: `FileTransferOptions` - configure transfer behavior
- `file_recv(remote, local, options)` - Receive file from device
  - `remote`: Remote device path
  - `local`: Local file path
  - `options`: `FileTransferOptions` - configure transfer behavior

**File Transfer Options:**
- `hold_timestamp(bool)` - Preserve file timestamps (`-a`)
- `sync_mode(bool)` - Only update if source is newer (`-sync`)
- `compress(bool)` - Compress during transfer (`-z`)
- `mode_sync(bool)` - Sync file permissions (`-m`)
- `debug_dir(bool)` - Transfer to/from debug app directory (`-b`)
- `cwd(path)` - Run the transfer relative to a working directory (`-cwd`)

### Usage Pattern

**Option 1: Select device first (Recommended)**
```rust
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
let devices = client.list_targets().await?;

// Connect to device - this re-establishes connection with device ID in handshake
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.connect_device(&devices[0]).await?;

// Now shell commands will be routed to the selected device
let output = client.shell("ls /data").await?;
```

**Option 2: Specify device per command**
```rust
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
let devices = client.list_targets().await?;

// Execute on specific device without selecting
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
let output = client.shell_on_device(&devices[0], "ls /data").await?;
```

**Option 3: Port forwarding**
```rust
use hdc_rs::{HdcClient, ForwardNode};

let mut client = HdcClient::connect("127.0.0.1:8710").await?;
let devices = client.list_targets().await?;
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.connect_device(&devices[0]).await?;

// Forward local TCP 8080 to device TCP 8081
client.fport(ForwardNode::Tcp(8080), ForwardNode::Tcp(8081)).await?;

// List all forwards
let tasks = client.fport_list().await?;
for task in tasks {
    println!("Forward: {}", task);
}

// Remove forward
client.fport_remove("tcp:8080 tcp:8081").await?;
```

**Option 4: App management**
```rust
use hdc_rs::{HdcClient, InstallOptions, UninstallOptions};

let mut client = HdcClient::connect("127.0.0.1:8710").await?;
let devices = client.list_targets().await?;
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.connect_device(&devices[0]).await?;

// Install app (replace if exists)
let opts = InstallOptions::new().replace(true);
client.install(&["app.hap"], opts).await?;

// Uninstall app (keep data)
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.connect_device(&devices[0]).await?;
let opts = UninstallOptions::new().keep_data(true);
client.uninstall("com.example.app", opts).await?;
```

**Option 5: Device logs (hilog)**
```rust
use hdc_rs::HdcClient;

let mut client = HdcClient::connect("127.0.0.1:8710").await?;
let devices = client.list_targets().await?;
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.connect_device(&devices[0]).await?;

// Get logs as buffered string
let logs = client.hilog(Some("-t app")).await?;
println!("App logs:\n{}", logs);

// Stream logs on a fresh channel
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.connect_device(&devices[0]).await?;
client
    .hilog_stream(None, |log_chunk| {
        println!("{}", log_chunk);
        false
    })
    .await?;
```

**Option 6: Monitor device connections**
```rust
use hdc_rs::HdcClient;
use std::time::Duration;

let mut client = HdcClient::connect("127.0.0.1:8710").await?;

// Wait for any device to connect (blocks until a device is available)
let device = client.wait_for_device().await?;
println!("Device connected: {}", device);

// Monitor device list changes in real-time
client.monitor_devices(Duration::from_secs(2), |devices| {
    println!("Device list updated: {} device(s)", devices.len());
    for device in devices {
        println!("  - {}", device);
    }
    true // Continue monitoring, return false to stop
}).await?;
```

**Option 7: File transfer**
```rust
use hdc_rs::{HdcClient, FileTransferOptions};

let mut client = HdcClient::connect("127.0.0.1:8710").await?;
let devices = client.list_targets().await?;
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.connect_device(&devices[0]).await?;

// Send file to device with options
let opts = FileTransferOptions::new()
    .hold_timestamp(true)   // Preserve timestamp
    .compress(true);         // Compress transfer
client.file_send("local.txt", "/data/local/tmp/remote.txt", opts).await?;

// Receive file from device
let mut client = HdcClient::connect("127.0.0.1:8710").await?;
client.connect_device(&devices[0]).await?;
let opts = FileTransferOptions::new().sync_mode(true);
client.file_recv("/data/local/tmp/remote.txt", "local.txt", opts).await?;
```

### Error Handling

All methods return `Result<T, HdcError>`. The library provides comprehensive error types:

```rust
use hdc_rs::{HdcClient, HdcError};

match client.shell("ls").await {
    Ok(output) => println!("{}", output),
    Err(HdcError::NotConnected) => eprintln!("Not connected to HDC server!"),
    Err(HdcError::Timeout) => eprintln!("Command timeout!"),
    Err(HdcError::DeviceNotFound(_)) => eprintln!("Device not found!"),
    Err(HdcError::Protocol(msg)) => eprintln!("Protocol error: {}", msg),
    Err(e) => eprintln!("Error: {}", e),
}
```

**Available Error Types:**
- `NotConnected` - Not connected to HDC server
- `ConnectionClosed` - HDC server closed the transport cleanly
- `Timeout` - Operation timeout
- `DeviceNotFound` - Target device not found
- `Protocol` - Protocol-level error
- `Io` - I/O error (network, file, etc.)
- `InvalidBanner` - Invalid server banner
- `CommandFailed` - Command execution failed

## 💻 Development

```
┌─────────────────────┐
│ 4 bytes: length     │ (big-endian u32)
├─────────────────────┤
│ N bytes: data       │
└─────────────────────┘
```

### Channel Handshake

1. Client connects to server
2. Server sends handshake with channel ID
3. Client responds with connect key
4. Connection established

## Current Status

### Implemented ✅

- TCP connection management
- Packet codec (length-prefixed protocol)
- Channel handshake
- Device management and target control commands
- Shell commands, file transfer, port forwarding, and app management
- Hilog and JDWP process tracking
- Error handling
- Async, blocking, and Python APIs

### Planned 🚧

- USB connection support
- Encryption (TLS-PSK)

## Development

### Prerequisites

- Rust 1.71 or later
- HDC server installed (from HarmonyOS SDK or OpenHarmony)
- A HarmonyOS/OpenHarmony device or emulator

### Building from Source

```bash
# Clone the repository
git clone https://github.com/oslo254804746/hdc-rs.git
cd hdc-rs

# Build the project
cargo build

# Build with release optimizations
cargo build --release

# Build with all features
cargo build --all-features
```

### Running Tests

```bash
# Run the pure Rust crate tests used by CI
cargo test -p hdc-rs --all-features

# Run the full workspace, including PyO3 bindings.
# Set PYO3_PYTHON explicitly so PyO3 does not depend on a local .venv path.
PYO3_PYTHON=$(command -v python3) cargo test --workspace --all-features

# Run specific test
cargo test test_name
```

The integration suites in `tests/integration_test.rs` and
`tests/forward_app_test.rs` are marked `#[ignore]` because they require a real
HDC server at `127.0.0.1:8710`; shell, file, forward, app, hilog, and monitor
flows also require a connected and authorized HarmonyOS/OpenHarmony device.
Run them explicitly when that environment is available:

```bash
cargo test --test integration_test -- --ignored
cargo test --test forward_app_test -- --ignored
```

### Enable Debug Logging

Set the `RUST_LOG` environment variable for detailed logging:

```bash
# Linux/macOS
RUST_LOG=hdc_rs=debug cargo run --example list_devices

# Windows PowerShell
$env:RUST_LOG="hdc_rs=debug"; cargo run --example list_devices

# Windows CMD
set RUST_LOG=hdc_rs=debug && cargo run --example list_devices
```

### Code Quality

```bash
# Format code
cargo fmt --all -- --check

# Lint with Clippy
PYO3_PYTHON=$(command -v python3) cargo clippy --workspace --all-features -- -D warnings

# Check without building
cargo check
```

### Documentation

```bash
# Generate and open documentation
cargo doc --open

# Generate documentation for all features
cargo doc --all-features --no-deps --open
```

## 🐛 Troubleshooting

### Common Issues

#### "No devices found"

**Solution:**
- Ensure HDC server is running: `hdc start`
- Check device connection: `hdc list targets`
- Verify device is authorized (check device screen for authorization prompt)
- For network devices: `hdc tconn <device_ip>:5555`

#### Connection timeout

**Symptoms:** `HdcError::Timeout` or connection hangs

**Solution:**
- Check if HDC server is listening on port 8710:
  - Windows: `netstat -ano | findstr 8710`
  - Linux/macOS: `netstat -an | grep 8710` or `lsof -i :8710`
- Restart HDC server: `hdc kill` then `hdc start`
- Check firewall settings

#### Protocol errors

**Symptoms:** `HdcError::Protocol` or unexpected responses

**Solution:**
- Ensure HDC server version compatibility (tested with 3.2.0+)
- Check server version: `hdc version` or `client.check_server().await?`
- Update HDC server to the latest version
- Enable debug logging to inspect protocol messages

#### File transfer fails

**Symptoms:** `HdcError::Io` during file operations

**Solution:**
- Verify file paths are correct
- Check device storage permissions
- Ensure target directory exists on device
- For large files, increase timeout or use compression

#### "Device not authorized"

**Solution:**
- Check device for authorization dialog
- Revoke and re-authorize: `hdc kill-server` then reconnect device
- Check device developer options are enabled

### Debug Tips

1. **Enable verbose logging:**
   ```bash
   RUST_LOG=hdc_rs=trace cargo run --example your_example
   ```

2. **Use the comprehensive example:**
   ```bash
   cargo run --example comprehensive
   ```

3. **Check HDC server logs:**
   - Server logs are typically in the HDC installation directory
   - Use `hdc -l 5` for verbose HDC logging

4. **Test with official HDC client:**
   ```bash
   hdc list targets
   hdc shell ls /data
   ```
   If the official client works but hdc-rs doesn't, please file an issue.

## 📊 Performance

- **Zero-copy parsing** with `bytes` crate
- **Async I/O** with Tokio for efficient concurrency
- **Connection pooling** support (reuse connections)
- **Streaming support** for large file transfers and log streaming

### Benchmarks

Typical performance on a modern system:

- Device listing: ~10-50ms
- Shell command: ~20-100ms (depends on command)
- File transfer: ~10-50 MB/s (depends on USB/network speed and device)

## 🗺️ Roadmap

### Current Status (v0.2.0)

- ✅ TCP connection management
- ✅ Async/await API
- ✅ Blocking API for FFI
- ✅ Device management (list, connect, monitor)
- ✅ Shell command execution
- ✅ File transfer (send/recv)
- ✅ Port forwarding (TCP, Unix sockets, JDWP, Ark)
- ✅ App management (install/uninstall)
- ✅ Log streaming (hilog)
- ✅ Python bindings (PyO3)

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

### Ways to Contribute

- 🐛 **Report bugs** - Open an issue with reproduction steps
- 💡 **Suggest features** - Share your ideas for improvements
- 📝 **Improve documentation** - Fix typos, add examples, clarify usage
- 🔧 **Submit pull requests** - Fix bugs or implement new features
- ⭐ **Star the project** - Show your support!

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Run formatting (`cargo fmt`)
6. Run linting (`cargo clippy`)
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

### Code Guidelines

- Follow Rust naming conventions and idioms
- Add tests for new functionality
- Update documentation for API changes
- Keep commits atomic and well-described
- Ensure all CI checks pass

### Testing

```bash
# Run the pure Rust crate tests used by CI
cargo test -p hdc-rs --all-features

# Run the full workspace, including PyO3 bindings
PYO3_PYTHON=$(command -v python3) cargo test --workspace --all-features

# Run specific test suite
cargo test --lib
cargo test --test integration_test -- --ignored
cargo test --test forward_app_test -- --ignored

# Run with verbose output
cargo test -- --nocapture
```

## 📄 License

This project is dual-licensed under:

- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- **Apache License 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

You may choose either license for your use.

## 🙏 Acknowledgments

- HarmonyOS/OpenHarmony development tools team for HDC
- Tokio project for excellent async runtime
- PyO3 project for Rust-Python bindings
- Rust community for amazing tools and libraries

## 📚 Resources

### Official Documentation

- [HDC Official Docs](https://gitee.com/openharmony/developtools_hdc)
- [HarmonyOS Developer Portal](https://developer.harmonyos.com/)
- [OpenHarmony Documentation](https://docs.openharmony.cn/)

### Related Projects

- [hdc-rs on crates.io](https://crates.io/crates/hdc-rs)
- [hdc-rs documentation](https://docs.rs/hdc-rs)
- [GitHub Repository](https://github.com/oslo254804746/hdc-rs)

### Community

- Report issues: [GitHub Issues](https://github.com/oslo254804746/hdc-rs/issues)
- Discussions: [GitHub Discussions](https://github.com/oslo254804746/hdc-rs/discussions)

## 📈 Project Status

![Rust Validation](https://github.com/oslo254804746/hdc-rs/actions/workflows/rust-validation.yml/badge.svg?branch=master)
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.71%2B-orange.svg)

**Version:** 0.2.0
**Status:** Active Development
**Stability:** Beta

---

Made with ❤️ by the HDC-RS contributors
