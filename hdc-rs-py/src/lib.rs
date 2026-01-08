use hdc_rs::app::{InstallOptions as RustInstallOptions, UninstallOptions as RustUninstallOptions};
use hdc_rs::blocking::HdcClient as RustHdcClient;
use hdc_rs::file::FileTransferOptions as RustFileTransferOptions;
use hdc_rs::forward::ForwardNode as RustForwardNode;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

/// Python wrapper for HDC client
#[pyclass]
struct HdcClient {
    inner: RustHdcClient,
}

#[pymethods]
impl HdcClient {
    /// Create a new HDC client and connect to the server
    ///
    /// Args:
    ///     addr: Server address (e.g., "127.0.0.1:8710")
    ///
    /// Returns:
    ///     HdcClient instance
    ///
    /// Example:
    ///     >>> client = HdcClient("127.0.0.1:8710")
    #[new]
    fn new(addr: &str) -> PyResult<Self> {
        let inner =
            RustHdcClient::connect(addr).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// List all connected devices
    ///
    /// Returns:
    ///     List of device IDs
    ///
    /// Example:
    ///     >>> devices = client.list_targets()
    ///     >>> print(devices)
    ///     ['FMR0223C13000649']
    fn list_targets(&mut self) -> PyResult<Vec<String>> {
        self.inner
            .list_targets()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Connect to a specific device
    ///
    /// Args:
    ///     device_id: Device identifier
    ///
    /// Example:
    ///     >>> client.connect_device("FMR0223C13000649")
    fn connect_device(&mut self, device_id: &str) -> PyResult<()> {
        self.inner
            .connect_device(device_id)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Execute a shell command on the device
    ///
    /// Args:
    ///     command: Shell command to execute
    ///
    /// Returns:
    ///     Command output as string
    ///
    /// Example:
    ///     >>> output = client.shell("ls -l /data")
    ///     >>> print(output)
    fn shell(&mut self, command: &str) -> PyResult<String> {
        self.inner
            .shell(command)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Send a file to device
    ///
    /// Args:
    ///     local_path: Local file path
    ///     remote_path: Remote file path on device
    ///     compress: Whether to compress during transfer (default: False)
    ///     hold_timestamp: Whether to hold/preserve file timestamp (default: False)
    ///     sync_mode: Only update newer files (default: False)
    ///     mode_sync: Enable mode sync (default: False)
    ///
    /// Returns:
    ///     Transfer result message
    ///
    /// Example:
    ///     >>> result = client.file_send("local.txt", "/data/local/tmp/remote.txt")
    ///     >>> print(result)
    #[pyo3(signature = (local_path, remote_path, compress=false, hold_timestamp=false, sync_mode=false, mode_sync=false))]
    fn file_send(
        &mut self,
        local_path: &str,
        remote_path: &str,
        compress: bool,
        hold_timestamp: bool,
        sync_mode: bool,
        mode_sync: bool,
    ) -> PyResult<String> {
        let options = RustFileTransferOptions::new()
            .compress(compress)
            .hold_timestamp(hold_timestamp)
            .sync_mode(sync_mode)
            .mode_sync(mode_sync);

        self.inner
            .file_send(local_path, remote_path, options)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Receive a file from device
    ///
    /// Args:
    ///     remote_path: Remote file path on device
    ///     local_path: Local file path
    ///     compress: Whether to compress during transfer (default: False)
    ///     hold_timestamp: Whether to hold/preserve file timestamp (default: False)
    ///     sync_mode: Only update newer files (default: False)
    ///     mode_sync: Enable mode sync (default: False)
    ///
    /// Returns:
    ///     Transfer result message
    ///
    /// Example:
    ///     >>> result = client.file_recv("/data/local/tmp/remote.txt", "local.txt")
    ///     >>> print(result)
    #[pyo3(signature = (remote_path, local_path, compress=false, hold_timestamp=false, sync_mode=false, mode_sync=false))]
    fn file_recv(
        &mut self,
        remote_path: &str,
        local_path: &str,
        compress: bool,
        hold_timestamp: bool,
        sync_mode: bool,
        mode_sync: bool,
    ) -> PyResult<String> {
        let options = RustFileTransferOptions::new()
            .compress(compress)
            .hold_timestamp(hold_timestamp)
            .sync_mode(sync_mode)
            .mode_sync(mode_sync);

        self.inner
            .file_recv(remote_path, local_path, options)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Create a forward port mapping (local -> device)
    ///
    /// Args:
    ///     local: Local forward node (e.g., "tcp:8080")
    ///     remote: Remote forward node (e.g., "tcp:8080")
    ///
    /// Returns:
    ///     Forward result message
    ///
    /// Example:
    ///     >>> result = client.fport("tcp:8080", "tcp:8080")
    ///     >>> print(result)
    fn fport(&mut self, local: &str, remote: &str) -> PyResult<String> {
        let local_node =
            RustForwardNode::parse(local).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let remote_node =
            RustForwardNode::parse(remote).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        self.inner
            .fport(local_node, remote_node)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Create a reverse port mapping (device -> local)
    ///
    /// Args:
    ///     remote: Remote forward node (e.g., "tcp:9090")
    ///     local: Local forward node (e.g., "tcp:9090")
    ///
    /// Returns:
    ///     Reverse forward result message
    ///
    /// Example:
    ///     >>> result = client.rport("tcp:9090", "tcp:9090")
    ///     >>> print(result)
    fn rport(&mut self, remote: &str, local: &str) -> PyResult<String> {
        let remote_node =
            RustForwardNode::parse(remote).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let local_node =
            RustForwardNode::parse(local).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        self.inner
            .rport(remote_node, local_node)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Remove a forward port mapping
    ///
    /// Args:
    ///     task_str: Task string (e.g., "tcp:8080 tcp:8080")
    ///
    /// Returns:
    ///     Remove result message
    ///
    /// Example:
    ///     >>> result = client.fport_remove("tcp:8080 tcp:8080")
    ///     >>> print(result)
    fn fport_remove(&mut self, task_str: &str) -> PyResult<String> {
        self.inner
            .fport_remove(task_str)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Install an application on the device
    ///
    /// Args:
    ///     packages: List of package paths (.hap or .hsp files)
    ///     replace: Replace existing application (default: False)
    ///     shared: Install shared bundle for multi-apps (default: False)
    ///
    /// Returns:
    ///     Install result message
    ///
    /// Example:
    ///     >>> result = client.install(["app.hap"], replace=True)
    ///     >>> print(result)
    #[pyo3(signature = (packages, replace=false, shared=false))]
    fn install(&mut self, packages: Vec<String>, replace: bool, shared: bool) -> PyResult<String> {
        let options = RustInstallOptions { replace, shared };
        let package_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
        self.inner
            .install(&package_refs, options)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Uninstall an application from the device
    ///
    /// Args:
    ///     package: Package name to uninstall
    ///     keep_data: Keep the data and cache directories (default: False)
    ///     shared: Remove shared bundle (default: False)
    ///
    /// Returns:
    ///     Uninstall result message
    ///
    /// Example:
    ///     >>> result = client.uninstall("com.example.app")
    ///     >>> print(result)
    #[pyo3(signature = (package, keep_data=false, shared=false))]
    fn uninstall(&mut self, package: &str, keep_data: bool, shared: bool) -> PyResult<String> {
        let options = RustUninstallOptions { keep_data, shared };

        self.inner
            .uninstall(package, options)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Get device logs (hilog)
    ///
    /// Args:
    ///     args: Optional hilog arguments (e.g., "-t MyTag")
    ///
    /// Returns:
    ///     Device logs as string
    ///
    /// Example:
    ///     >>> logs = client.hilog()
    ///     >>> print(logs)
    ///     >>> # With filter
    ///     >>> logs = client.hilog("-t MyTag")
    fn hilog(&mut self, args: Option<&str>) -> PyResult<String> {
        self.inner
            .hilog(args)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Wait for a device to be connected
    ///
    /// Returns:
    ///     Device ID of the connected device
    ///
    /// Example:
    ///     >>> device_id = client.wait_for_device()
    ///     >>> print(f"Device connected: {device_id}")
    fn wait_for_device(&mut self) -> PyResult<String> {
        self.inner
            .wait_for_device()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Stream device logs continuously with callback
    ///
    /// Args:
    ///     callback: Python callable that receives log chunks (str). Return True to continue, False to stop.
    ///     args: Optional hilog arguments (e.g., "-t MyTag")
    ///
    /// Example:
    ///     >>> def log_handler(log_chunk):
    ///     ...     print(log_chunk, end='')
    ///     ...     return True  # Continue streaming
    ///     >>> client.hilog_stream(log_handler)
    ///     >>> # With filter
    ///     >>> client.hilog_stream(log_handler, args="-t MyTag")
    #[pyo3(signature = (callback, args=None))]
    fn hilog_stream(&mut self, callback: PyObject, args: Option<&str>) -> PyResult<()> {
        Python::with_gil(|py| {
            self.inner
                .hilog_stream(args, |log_chunk: &str| {
                    // Call Python callback with log chunk
                    let result = callback.call1(py, (log_chunk,));

                    match result {
                        Ok(ret) => {
                            // Check if callback returned True/False
                            ret.extract::<bool>(py).unwrap_or(false)
                        }
                        Err(e) => {
                            // Print error but don't stop streaming
                            eprintln!("Callback error: {}", e);
                            false
                        }
                    }
                })
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Monitor device list changes with callback
    ///
    /// Continuously polls the device list and calls the callback when changes are detected.
    /// Note: HDC doesn't have a native "track-devices" command like adb, so this uses polling.
    ///
    /// Args:
    ///     callback: Python callable that receives list of device IDs. Return True to continue, False to stop.
    ///     interval_secs: Polling interval in seconds (default: 2, recommended: 1-3 seconds)
    ///
    /// Example:
    ///     >>> def device_monitor(devices):
    ///     ...     print(f"Devices: {devices}")
    ///     ...     return True  # Continue monitoring
    ///     >>> client.monitor_devices(device_monitor, interval_secs=2)
    #[pyo3(signature = (callback, interval_secs=2))]
    fn monitor_devices(&mut self, callback: PyObject, interval_secs: u64) -> PyResult<()> {
        Python::with_gil(|py| {
            self.inner
                .monitor_devices(interval_secs, |devices: &[String]| {
                    // Convert to Python list
                    let py_list = devices.to_vec();

                    // Call Python callback with device list
                    let result = callback.call1(py, (py_list,));

                    match result {
                        Ok(ret) => {
                            // Check if callback returned True/False
                            ret.extract::<bool>(py).unwrap_or(false)
                        }
                        Err(e) => {
                            // Print error but don't stop monitoring
                            eprintln!("Callback error: {}", e);
                            false
                        }
                    }
                })
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        })
    }
}

/// HDC Python module - HarmonyOS Device Connector client library
#[pymodule]
fn hdc_rs_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HdcClient>()?;
    Ok(())
}
