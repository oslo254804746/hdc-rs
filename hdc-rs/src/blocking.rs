//! Blocking (synchronous) API for HDC client
//!
//! This module provides a synchronous wrapper around the async HDC client,
//! suitable for use in synchronous contexts or FFI bindings (like PyO3).
//!
//! # Example
//!
//! ```no_run
//! use hdc_rs::blocking::HdcClient;
//!
//! let mut client = HdcClient::connect("127.0.0.1:8710")?;
//! let devices = client.list_targets()?;
//! println!("Devices: {:?}", devices);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::{app::InstallOptions, app::UninstallOptions, file::FileTransferOptions, Result};

/// Blocking HDC client
///
/// This is a synchronous wrapper around the async [`crate::HdcClient`].
/// It creates a tokio runtime internally to execute async operations.
pub struct HdcClient {
    runtime: tokio::runtime::Runtime,
    inner: crate::HdcClient,
}

impl HdcClient {
    /// Connect to HDC server
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    ///
    /// let client = HdcClient::connect("127.0.0.1:8710")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn connect(addr: &str) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(crate::HdcError::Io)?;

        let inner = runtime.block_on(crate::HdcClient::connect(addr))?;

        Ok(Self { runtime, inner })
    }

    /// List all connected devices
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    /// let devices = client.list_targets()?;
    /// for device in devices {
    ///     println!("Device: {}", device);
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn list_targets(&mut self) -> Result<Vec<String>> {
        self.runtime.block_on(self.inner.list_targets())
    }

    /// Connect to a specific device
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    /// let devices = client.list_targets()?;
    /// if !devices.is_empty() {
    ///     client.connect_device(&devices[0])?;
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn connect_device(&mut self, device_id: &str) -> Result<()> {
        self.runtime.block_on(self.inner.connect_device(device_id))
    }

    /// Execute a shell command on the device
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    /// let devices = client.list_targets()?;
    /// client.connect_device(&devices[0])?;
    ///
    /// let output = client.shell("ls -l")?;
    /// println!("Output: {}", output);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn shell(&mut self, command: &str) -> Result<String> {
        self.runtime.block_on(self.inner.shell(command))
    }

    /// Create a forward port mapping (local -> device)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    /// use hdc_rs::forward::ForwardNode;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    /// let devices = client.list_targets()?;
    /// client.connect_device(&devices[0])?;
    ///
    /// let local = ForwardNode::Tcp(8080);
    /// let remote = ForwardNode::Tcp(8080);
    /// let result = client.fport(local, remote)?;
    /// println!("Forward result: {}", result);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn fport(
        &mut self,
        local: crate::forward::ForwardNode,
        remote: crate::forward::ForwardNode,
    ) -> Result<String> {
        self.runtime.block_on(self.inner.fport(local, remote))
    }

    /// Create a reverse port mapping (device -> local)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    /// use hdc_rs::forward::ForwardNode;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    /// let devices = client.list_targets()?;
    /// client.connect_device(&devices[0])?;
    ///
    /// let remote = ForwardNode::Tcp(9090);
    /// let local = ForwardNode::Tcp(9090);
    /// let result = client.rport(remote, local)?;
    /// println!("Reverse forward result: {}", result);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn rport(
        &mut self,
        remote: crate::forward::ForwardNode,
        local: crate::forward::ForwardNode,
    ) -> Result<String> {
        self.runtime.block_on(self.inner.rport(remote, local))
    }

    /// Remove a forward port mapping
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    /// let devices = client.list_targets()?;
    /// client.connect_device(&devices[0])?;
    ///
    /// // Remove forward using task string (e.g., "tcp:8080 tcp:8080")
    /// client.fport_remove("tcp:8080 tcp:8080")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn fport_remove(&mut self, task_str: &str) -> Result<String> {
        self.runtime.block_on(self.inner.fport_remove(task_str))
    }

    /// Install an application on the device
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    /// use hdc_rs::app::InstallOptions;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    /// let devices = client.list_targets()?;
    /// client.connect_device(&devices[0])?;
    ///
    /// let packages = vec!["app.hap"];
    /// let options = InstallOptions::new();
    /// let result = client.install(&packages, options)?;
    /// println!("Install result: {}", result);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn install(&mut self, packages: &[&str], options: InstallOptions) -> Result<String> {
        self.runtime.block_on(self.inner.install(packages, options))
    }

    /// Uninstall an application from the device
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    /// use hdc_rs::app::UninstallOptions;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    /// let devices = client.list_targets()?;
    /// client.connect_device(&devices[0])?;
    ///
    /// let options = UninstallOptions::new();
    /// let result = client.uninstall("com.example.app", options)?;
    /// println!("Uninstall result: {}", result);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn uninstall(&mut self, package: &str, options: UninstallOptions) -> Result<String> {
        self.runtime
            .block_on(self.inner.uninstall(package, options))
    }

    /// Send a file to the device
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    /// use hdc_rs::file::FileTransferOptions;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    /// let devices = client.list_targets()?;
    /// client.connect_device(&devices[0])?;
    ///
    /// let options = FileTransferOptions::default();
    /// let result = client.file_send("local.txt", "/data/local/tmp/remote.txt", options)?;
    /// println!("Transfer result: {}", result);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn file_send(
        &mut self,
        local_path: &str,
        remote_path: &str,
        options: FileTransferOptions,
    ) -> Result<String> {
        self.runtime
            .block_on(self.inner.file_send(local_path, remote_path, options))
    }

    /// Receive a file from the device
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    /// use hdc_rs::file::FileTransferOptions;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    /// let devices = client.list_targets()?;
    /// client.connect_device(&devices[0])?;
    ///
    /// let options = FileTransferOptions::default();
    /// let result = client.file_recv("/data/local/tmp/remote.txt", "local.txt", options)?;
    /// println!("Transfer result: {}", result);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn file_recv(
        &mut self,
        remote_path: &str,
        local_path: &str,
        options: FileTransferOptions,
    ) -> Result<String> {
        self.runtime
            .block_on(self.inner.file_recv(remote_path, local_path, options))
    }

    /// Get device logs (hilog) with buffering
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    /// let devices = client.list_targets()?;
    /// client.connect_device(&devices[0])?;
    ///
    /// // Get all logs
    /// let logs = client.hilog(None)?;
    /// println!("Logs: {}", logs);
    ///
    /// // Get logs with filter
    /// let logs = client.hilog(Some("-t MyTag"))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn hilog(&mut self, args: Option<&str>) -> Result<String> {
        self.runtime.block_on(self.inner.hilog(args))
    }

    /// Wait for a device to be connected
    ///
    /// This will block until a device is found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    ///
    /// // Wait for a device
    /// let device = client.wait_for_device()?;
    /// println!("Device found: {}", device);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn wait_for_device(&mut self) -> Result<String> {
        self.runtime.block_on(self.inner.wait_for_device())
    }

    /// Stream device logs (hilog) with callback
    ///
    /// This method continuously streams logs from the device and calls the callback
    /// function for each log chunk received. The callback should return `true` to
    /// continue streaming or `false` to stop.
    ///
    /// # Arguments
    /// * `args` - Optional hilog command arguments (e.g., "-t MyTag" for filtering)
    /// * `callback` - Function called for each log chunk. Return `true` to continue, `false` to stop.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    /// let devices = client.list_targets()?;
    /// client.connect_device(&devices[0])?;
    ///
    /// // Stream all logs
    /// client.hilog_stream(None, |log_chunk| {
    ///     print!("{}", log_chunk);
    ///     true // Continue streaming
    /// })?;
    ///
    /// // Stream with filter
    /// client.hilog_stream(Some("-t MyTag"), |log_chunk| {
    ///     print!("{}", log_chunk);
    ///     true
    /// })?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn hilog_stream<F>(&mut self, args: Option<&str>, callback: F) -> Result<()>
    where
        F: FnMut(&str) -> bool,
    {
        self.runtime
            .block_on(self.inner.hilog_stream(args, callback))
    }

    /// Monitor device list changes with callback
    ///
    /// This function continuously polls the device list and calls the callback
    /// when changes are detected. The callback should return `true` to continue
    /// monitoring or `false` to stop.
    ///
    /// Note: HDC doesn't have a native "track-devices" command like adb,
    /// so this implementation uses polling to detect changes.
    ///
    /// # Arguments
    /// * `interval` - Polling interval in seconds (recommended: 1-3 seconds)
    /// * `callback` - Function called when device list changes. Return `true` to continue, `false` to stop.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_rs::blocking::HdcClient;
    ///
    /// let mut client = HdcClient::connect("127.0.0.1:8710")?;
    ///
    /// // Monitor device changes every 2 seconds
    /// client.monitor_devices(2, |devices| {
    ///     println!("Device list changed:");
    ///     for device in devices {
    ///         println!("  - {}", device);
    ///     }
    ///     true // Continue monitoring
    /// })?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn monitor_devices<F>(&mut self, interval_secs: u64, callback: F) -> Result<()>
    where
        F: FnMut(&[String]) -> bool,
    {
        let interval = std::time::Duration::from_secs(interval_secs);
        self.runtime
            .block_on(self.inner.monitor_devices(interval, callback))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires HDC server running
    fn test_blocking_client_creation() {
        let result = HdcClient::connect("127.0.0.1:8710");
        assert!(result.is_ok());
    }

    #[test]
    #[ignore] // Requires HDC server running
    fn test_blocking_list_targets() {
        let mut client = HdcClient::connect("127.0.0.1:8710").unwrap();
        let result = client.list_targets();
        assert!(result.is_ok());
    }
}
