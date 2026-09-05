use hdc_rs::app::{InstallOptions as RustInstallOptions, UninstallOptions as RustUninstallOptions};
use hdc_rs::blocking::HdcClient as RustHdcClient;
use hdc_rs::device::{TargetBootMode as RustTargetBootMode, TargetMode as RustTargetMode};
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

    /// List connected devices with verbose HDC output.
    fn list_targets_verbose(&mut self) -> PyResult<String> {
        self.inner
            .list_targets_verbose()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Check HDC server state/version.
    fn check_server(&mut self) -> PyResult<String> {
        self.inner
            .check_server()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Get HDC version information.
    fn version(&mut self) -> PyResult<String> {
        self.inner
            .version()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Get HDC help text.
    #[pyo3(signature = (verbose=false))]
    fn help(&mut self, verbose: bool) -> PyResult<String> {
        self.inner
            .help(verbose)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Ask HDC server to discover targets.
    fn discover(&mut self) -> PyResult<String> {
        self.inner
            .discover()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Check target device state.
    fn check_device(&mut self, connect_key: Option<&str>) -> PyResult<String> {
        self.inner
            .check_device(connect_key)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Connect to a TCP/manual target by connect key.
    fn target_connect(&mut self, key: &str) -> PyResult<String> {
        self.inner
            .target_connect(key)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Disconnect a TCP/manual target by connect key.
    fn target_disconnect(&mut self, key: &str) -> PyResult<String> {
        self.inner
            .target_disconnect(key)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Select any available target.
    fn connect_any(&mut self) -> PyResult<String> {
        self.inner
            .connect_any()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Reconnect the current or specified target.
    fn reconnect_target(&mut self, connect_key: Option<&str>) -> PyResult<String> {
        self.inner
            .reconnect_target(connect_key)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Mount the target filesystem.
    fn target_mount(&mut self) -> PyResult<String> {
        self.inner
            .target_mount()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Boot the target. Pass values such as "recovery" or "bootloader".
    fn target_boot(&mut self, mode: Option<&str>) -> PyResult<String> {
        self.inner
            .target_boot(mode.map(RustTargetBootMode::from))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Switch daemon privilege mode.
    fn smode(&mut self, enable_root: bool) -> PyResult<String> {
        self.inner
            .smode(enable_root)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Switch target to USB mode.
    fn tmode_usb(&mut self) -> PyResult<String> {
        self.inner
            .tmode(RustTargetMode::Usb)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Switch target to TCP port mode.
    fn tmode_port(&mut self, port: Option<u16>) -> PyResult<String> {
        self.inner
            .tmode(RustTargetMode::Port(port))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Close target TCP port mode.
    fn tmode_port_close(&mut self) -> PyResult<String> {
        self.inner
            .tmode(RustTargetMode::PortClose)
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
    ///     debug_dir: Send to debug application directory (default: False)
    ///     cwd: Working directory for transfer (default: None)
    ///
    /// Returns:
    ///     Transfer result message
    ///
    /// Example:
    ///     >>> result = client.file_send("local.txt", "/data/local/tmp/remote.txt")
    ///     >>> print(result)
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (local_path, remote_path, compress=false, hold_timestamp=false, sync_mode=false, mode_sync=false, debug_dir=false, cwd=None))]
    fn file_send(
        &mut self,
        local_path: &str,
        remote_path: &str,
        compress: bool,
        hold_timestamp: bool,
        sync_mode: bool,
        mode_sync: bool,
        debug_dir: bool,
        cwd: Option<&str>,
    ) -> PyResult<String> {
        let mut options = RustFileTransferOptions::new()
            .compress(compress)
            .hold_timestamp(hold_timestamp)
            .sync_mode(sync_mode)
            .mode_sync(mode_sync)
            .debug_dir(debug_dir);

        if let Some(cwd) = cwd {
            options = options.cwd(cwd);
        }

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
    ///     debug_dir: Receive from debug application directory (default: False)
    ///     cwd: Working directory for transfer (default: None)
    ///
    /// Returns:
    ///     Transfer result message
    ///
    /// Example:
    ///     >>> result = client.file_recv("/data/local/tmp/remote.txt", "local.txt")
    ///     >>> print(result)
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (remote_path, local_path, compress=false, hold_timestamp=false, sync_mode=false, mode_sync=false, debug_dir=false, cwd=None))]
    fn file_recv(
        &mut self,
        remote_path: &str,
        local_path: &str,
        compress: bool,
        hold_timestamp: bool,
        sync_mode: bool,
        mode_sync: bool,
        debug_dir: bool,
        cwd: Option<&str>,
    ) -> PyResult<String> {
        let mut options = RustFileTransferOptions::new()
            .compress(compress)
            .hold_timestamp(hold_timestamp)
            .sync_mode(sync_mode)
            .mode_sync(mode_sync)
            .debug_dir(debug_dir);

        if let Some(cwd) = cwd {
            options = options.cwd(cwd);
        }

        self.inner
            .file_recv(remote_path, local_path, options)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Create a forward port mapping (local -> device)
    ///
    /// The terminal task drains its channel and disconnects the client on return.
    /// Create a fresh client before another terminal task.
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
    /// The terminal task drains its channel and disconnects the client on return.
    /// Create a fresh client before another terminal task.
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

    /// List forward and reverse port mappings using fport ls.
    fn fport_list(&mut self) -> PyResult<Vec<String>> {
        self.inner
            .fport_list()
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
    ///     cwd: Working directory (default: None)
    ///     wait_time: Wait time in seconds (default: None)
    ///     user_id: User ID (default: None)
    ///     list_options: List install options (default: False)
    ///     grant_permissions: Grant permissions after install (default: False)
    ///
    /// Returns:
    ///     Install result message
    ///
    /// Example:
    ///     >>> result = client.install(["app.hap"], replace=True)
    ///     >>> print(result)
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (packages, replace=false, shared=false, cwd=None, wait_time=None, user_id=None, list_options=false, grant_permissions=false))]
    fn install(
        &mut self,
        packages: Vec<String>,
        replace: bool,
        shared: bool,
        cwd: Option<&str>,
        wait_time: Option<u64>,
        user_id: Option<&str>,
        list_options: bool,
        grant_permissions: bool,
    ) -> PyResult<String> {
        let mut options = RustInstallOptions::new()
            .replace(replace)
            .shared(shared)
            .list_options(list_options)
            .grant_permissions(grant_permissions);
        if let Some(cwd) = cwd {
            options = options.cwd(cwd);
        }
        if let Some(wait_time) = wait_time {
            options = options.wait_time(wait_time);
        }
        if let Some(user_id) = user_id {
            options = options.user_id(user_id);
        }
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
    ///     module_name: Module name option (default: None)
    ///     version_code: Version code option (default: None)
    ///     user_id: User ID (default: None)
    ///     list_options: List uninstall options (default: False)
    ///
    /// Returns:
    ///     Uninstall result message
    ///
    /// Example:
    ///     >>> result = client.uninstall("com.example.app")
    ///     >>> print(result)
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (package, keep_data=false, shared=false, module_name=None, version_code=None, user_id=None, list_options=false))]
    fn uninstall(
        &mut self,
        package: &str,
        keep_data: bool,
        shared: bool,
        module_name: Option<&str>,
        version_code: Option<&str>,
        user_id: Option<&str>,
        list_options: bool,
    ) -> PyResult<String> {
        let mut options = RustUninstallOptions::new()
            .keep_data(keep_data)
            .shared(shared)
            .list_options(list_options);
        if let Some(module_name) = module_name {
            options = options.module_name(module_name);
        }
        if let Some(version_code) = version_code {
            options = options.version_code(version_code);
        }
        if let Some(user_id) = user_id {
            options = options.user_id(user_id);
        }

        self.inner
            .uninstall(package, options)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Get device logs (hilog)
    ///
    /// Captures for at most two seconds, ending earlier after 500 ms without a
    /// complete packet once output has arrived. Closes the channel on return.
    /// Use hilog_stream for continuous output.
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
    #[pyo3(signature = (args=None))]
    fn hilog(&mut self, args: Option<&str>) -> PyResult<String> {
        self.inner
            .hilog(args)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// List debug/JDWP process identifiers.
    fn jpid(&mut self) -> PyResult<Vec<String>> {
        self.inner
            .jpid()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Track debug/JDWP process changes with a callback.
    #[pyo3(signature = (callback, include_release=false, pid_only=false))]
    fn track_jpid(
        &mut self,
        callback: PyObject,
        include_release: bool,
        pid_only: bool,
    ) -> PyResult<()> {
        Python::with_gil(|py| {
            self.inner
                .track_jpid(include_release, pid_only, |chunk: &str| {
                    let result = callback.call1(py, (chunk,));

                    match result {
                        Ok(ret) => ret.extract::<bool>(py).unwrap_or(false),
                        Err(e) => {
                            eprintln!("Callback error: {}", e);
                            false
                        }
                    }
                })
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        })
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
