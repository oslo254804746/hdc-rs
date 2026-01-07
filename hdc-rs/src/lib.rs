//! # HDC Rust Client Library
//!
//! A Rust implementation of the HarmonyOS Device Connector (HDC) client.
//! This library provides a type-safe, async interface to communicate with HDC servers.
//!
//! ## Features
//!
//! - **Async/await** - Built on Tokio for efficient async I/O
//! - **Device Management** - List, connect, and monitor devices
//! - **Shell Commands** - Execute commands on devices  
//! - **Port Forwarding** - TCP, Unix sockets, JDWP, Ark debugger support
//! - **App Management** - Install and uninstall applications (.hap, .hsp)
//! - **File Transfer** - Send and receive files with options
//! - **Log Streaming** - Read device logs (hilog)
//! - **Type-safe API** - Rust's type system ensures correctness
//! - **Error Handling** - Comprehensive error types with context
//!
//! ## Quick Start
//!
//! ```no_run
//! use hdc_rs::HdcClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to HDC server
//!     let mut client = HdcClient::connect("127.0.0.1:8710").await?;
//!     
//!     // List connected devices
//!     let devices = client.list_targets().await?;
//!     println!("Devices: {:?}", devices);
//!     
//!     if !devices.is_empty() {
//!         // Select and connect to first device
//!         client.connect_device(&devices[0]).await?;
//!         
//!         // Execute shell command
//!         let output = client.shell("ls -l /data").await?;
//!         println!("{}", output);
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Module Organization
//!
//! - [`client`] - Main HDC client implementation
//! - [`blocking`] - Synchronous/blocking API (requires `blocking` feature)
//! - [`app`] - Application management types and options
//! - [`file`] - File transfer types and options
//! - [`forward`] - Port forwarding types
//! - [`protocol`] - HDC protocol implementation
//! - [`error`] - Error types
//!
//! ## Blocking API
//!
//! For synchronous contexts or FFI bindings (e.g., PyO3), use the blocking module:
//!
//! ```no_run
//! use hdc_rs::blocking::HdcClient;
//!
//! let mut client = HdcClient::connect("127.0.0.1:8710")?;
//! let devices = client.list_targets()?;
//! println!("Devices: {:?}", devices);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Examples
//!
//! See the `examples/` directory for comprehensive examples:
//!
//! - `list_devices` - List connected devices
//! - `device_monitor` - Monitor device connections
//! - `simple_shell` - Interactive shell
//! - `forward_demo` - Port forwarding
//! - `file_demo` - File transfer
//! - `app_demo` - App installation
//! - `hilog_demo` - Device logs
//! - `comprehensive` - All features

pub mod app;
#[cfg(feature = "blocking")]
pub mod blocking;
pub mod client;
pub mod error;
pub mod file;
pub mod forward;
pub mod protocol;

pub use app::{InstallOptions, UninstallOptions};
pub use client::HdcClient;
pub use error::{HdcError, Result};
pub use file::{FileTransferDirection, FileTransferOptions};
pub use forward::{ForwardNode, ForwardTask};
