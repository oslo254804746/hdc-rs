//! HDC client implementation

use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::error::{HdcError, Result};
use crate::protocol::{ChannelHandShake, HdcCommand, PacketCodec};

/// Default connection timeout
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// HDC client for communicating with HDC server
pub struct HdcClient {
    /// TCP stream to HDC server
    stream: Option<TcpStream>,
    /// Server address
    address: String,
    /// Packet codec for encoding/decoding
    codec: PacketCodec,
    /// Channel ID assigned by server
    channel_id: u32,
    /// Whether handshake is complete
    handshake_ok: bool,
    /// Current connect key (device identifier)
    connect_key: Option<String>,
}

impl HdcClient {
    /// Create a new HDC client (not connected)
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            stream: None,
            address: address.into(),
            codec: PacketCodec::new(),
            channel_id: 0,
            handshake_ok: false,
            connect_key: None,
        }
    }

    /// Connect to HDC server
    pub async fn connect(address: impl Into<String>) -> Result<Self> {
        let mut client = Self::new(address);
        client.connect_internal().await?;
        Ok(client)
    }

    /// Internal connection method
    async fn connect_internal(&mut self) -> Result<()> {
        info!("Connecting to HDC server at {}", self.address);

        let stream = timeout(DEFAULT_TIMEOUT, TcpStream::connect(&self.address))
            .await
            .map_err(|_| HdcError::Timeout)?
            .map_err(HdcError::Io)?;

        info!("Connected to HDC server");
        self.stream = Some(stream);

        // Perform channel handshake
        self.perform_handshake(None).await?;

        Ok(())
    }

    /// Perform channel handshake with server
    async fn perform_handshake(&mut self, connect_key: Option<&str>) -> Result<()> {
        let stream = self.stream.as_mut().ok_or(HdcError::NotConnected)?;

        info!("Starting channel handshake");

        // Step 1: Read handshake from server
        let handshake_data = self.codec.read_packet(stream).await?;
        let received_size = handshake_data.len();
        debug!("Received handshake data: {} bytes", received_size);

        let mut handshake = ChannelHandShake::from_bytes(&handshake_data)?;

        // Step 2: Verify banner
        handshake.verify_banner()?;
        info!("Banner verified: {:?}", &handshake.banner[..8]);

        // Step 3: Extract channel ID
        self.channel_id = handshake.get_channel_id();
        info!("Assigned channel ID: {}", self.channel_id);

        // Step 4: Check features
        let is_stable = handshake.is_stable_buf();
        debug!("Server stable buffer mode: {}", is_stable);

        // Step 5: Set connect key and send response
        if let Some(key) = connect_key {
            handshake.set_connect_key(key);
            self.connect_key = Some(key.to_string());
            info!("Using connect key: {}", key);
        } else {
            // Empty connect key for initial connection
            handshake.set_connect_key("");
        }

        // Send handshake response with same format as received
        // If server sent 44 bytes (without version), respond with 44 bytes
        // If server sent 108 bytes (with version), respond with 108 bytes
        let response = if received_size >= ChannelHandShake::SIZE {
            debug!("Sending full handshake response (108 bytes)");
            handshake.to_bytes()
        } else {
            debug!("Sending handshake response without version (44 bytes)");
            handshake.to_bytes_without_version()
        };

        self.codec.write_packet(stream, &response).await?;

        self.handshake_ok = true;
        info!("Channel handshake completed successfully");

        Ok(())
    }

    /// Get the channel ID
    pub fn channel_id(&self) -> u32 {
        self.channel_id
    }

    /// Check if handshake is complete
    pub fn is_connected(&self) -> bool {
        self.handshake_ok && self.stream.is_some()
    }

    /// Send raw command string to server
    ///
    /// This is used for simple commands like "list targets", "shell ls", etc.
    pub async fn send_command(&mut self, command: &str) -> Result<()> {
        if !self.is_connected() {
            return Err(HdcError::NotConnected);
        }
        if let Some(ref mut tcp_stream) = self.stream {
            debug!("Sending command: {}", command);

            // For simple commands, just send the command string
            let cmd_bytes = command.as_bytes();
            self.codec.write_packet(tcp_stream, cmd_bytes).await?;

            return Ok(());
        }
        Err(HdcError::NotConnected)
    }

    /// Read response from server
    pub async fn read_response(&mut self) -> Result<Vec<u8>> {
        if !self.is_connected() {
            return Err(HdcError::NotConnected);
        }

        let stream = self.stream.as_mut().unwrap();
        let data = self.codec.read_packet(stream).await?;

        Ok(data)
    }

    /// Read response as string
    pub async fn read_response_string(&mut self) -> Result<String> {
        let data = self.read_response().await?;

        if data.is_empty() {
            return Ok(String::new());
        }

        // Check if there's a command prefix (2 bytes)
        if data.len() >= 2 {
            let cmd_code = u16::from_le_bytes([data[0], data[1]]);
            if let Some(cmd) = HdcCommand::from_u16(cmd_code) {
                debug!("Response has command prefix: {:?}", cmd);
                // Skip command bytes
                return Ok(String::from_utf8(data[2..].to_vec())?);
            }
        }

        Ok(String::from_utf8(data)?)
    }

    /// Execute a shell command and return output
    ///
    /// If a device has been selected via `connect_device()`, the command will be
    /// executed on that device (the device ID is set in the channel's connectKey
    /// during handshake). Otherwise, HDC server will return an error asking
    /// to specify a device.
    ///
    /// Note: Each shell command uses up the current channel. After execution,
    /// the connection is automatically re-established if a device was connected.
    pub async fn shell(&mut self, cmd: &str) -> Result<String> {
        info!("Executing shell command: {}", cmd);

        // Save the current connect key before executing
        let device_id = self.connect_key.clone();

        // Command format is just "shell <cmd>"
        // Device targeting is done via the connectKey in handshake, not via -t parameter
        let full_cmd = format!("shell {}", cmd);

        self.send_command(&full_cmd).await?;

        // For shell commands, HDC server sends a single response packet with raw output data
        // No command code prefix, just the plain output
        let output = match timeout(Duration::from_secs(5), self.read_response()).await {
            Ok(Ok(data)) => {
                debug!("Shell response: {} bytes", data.len());
                String::from_utf8_lossy(&data).to_string()
            }
            Ok(Err(e)) => {
                debug!("Error reading shell response: {}", e);
                return Err(e);
            }
            Err(_) => {
                warn!("Timeout reading shell response");
                return Err(HdcError::Timeout);
            }
        };

        // Shell command consumes the channel - reconnect if we had a device
        if let Some(device) = device_id {
            debug!("Reconnecting to device after shell command");
            if let Err(e) = self.connect_device(&device).await {
                warn!("Failed to reconnect after shell: {}", e);
                // Don't fail the shell command itself, just log the warning
            }
        }

        Ok(output)
    }

    /// List connected devices/targets
    pub async fn list_targets(&mut self) -> Result<Vec<String>> {
        info!("Listing targets");

        self.send_command("list targets").await?;

        let response = self.read_response_string().await?;
        debug!("List targets response: {}", response);

        // Parse device list (format: one device per line)
        let devices: Vec<String> = response
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect();

        info!("Found {} device(s)", devices.len());
        Ok(devices)
    }

    // pub async fn get_device_stream(&self, device_id: &str) -> Result<HdcClient>{
    //     let stream = timeout(DEFAULT_TIMEOUT, TcpStream::connect(&self.address))
    //         .await
    //         .map_err(|_| HdcError::Timeout)?
    //         .map_err(HdcError::Io)?;
    //     let mut  client = HdcClient{
    //         stream: Some(stream),
    //         address: self.address.clone(),
    //         codec: PacketCodec::new(),
    //         channel_id: 0,
    //         handshake_ok: false,
    //         connect_key: None,
    //     };
    //     client.perform_handshake(Some(device_id)).await?;
    //     Ok(client)
    // }

    /// Connect to a specific device
    ///
    /// This re-establishes the connection with the specified device ID in the handshake.
    /// After calling this, all commands will be executed on the specified device.
    pub async fn connect_device(&mut self, device_id: &str) -> Result<()> {
        info!("Connecting to device: {}", device_id);

        // Close existing connection
        if self.stream.is_some() {
            debug!("Closing existing connection");
            self.stream = None;
            self.handshake_ok = false;
        }

        // Reconnect with new device ID
        let stream = timeout(DEFAULT_TIMEOUT, TcpStream::connect(&self.address))
            .await
            .map_err(|_| HdcError::Timeout)?
            .map_err(HdcError::Io)?;

        self.stream = Some(stream);

        // Perform handshake with connect key
        self.perform_handshake(Some(device_id)).await?;
        self.connect_key = Some(device_id.to_string());

        Ok(())
    }

    /// Check server version
    pub async fn check_server(&mut self) -> Result<String> {
        info!("Checking server version");

        self.send_command("checkserver").await?;
        let response = self.read_response_string().await?;

        debug!("Server version: {}", response);
        Ok(response)
    }

    /// Execute a command on a specific device
    ///
    /// This is a convenience method that:
    /// 1. Connects to the specified device (re-handshake with connectKey)
    /// 2. Executes the command
    ///
    /// Note: This changes the client's current device setting.
    pub async fn target_command(&mut self, device_id: &str, cmd: &str) -> Result<String> {
        info!("Executing target command on {}: {}", device_id, cmd);

        // Connect to device first (sets connectKey in handshake)
        self.connect_device(device_id).await?;

        // Send command directly
        self.send_command(cmd).await?;
        let output = self.read_response_string().await?;
        Ok(output)
    }

    /// Execute a shell command on a specific device (convenience method)
    ///
    /// This connects to the device and executes: `shell <cmd>`
    pub async fn shell_on_device(&mut self, device_id: &str, cmd: &str) -> Result<String> {
        // Connect to device first
        self.connect_device(device_id).await?;

        // Execute shell command
        self.shell(cmd).await
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<()> {
        if let Some(stream) = self.stream.take() {
            info!("Closing connection");
            drop(stream);
            self.handshake_ok = false;
        }
        Ok(())
    }

    // ========== Forward Commands ==========

    /// Create a port forward (fport)
    ///
    /// Forward local traffic to remote device.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::{HdcClient, ForwardNode};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// // Forward local TCP 8080 to device TCP 8081
    /// client.fport(ForwardNode::Tcp(8080), ForwardNode::Tcp(8081)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fport(
        &mut self,
        local: crate::forward::ForwardNode,
        remote: crate::forward::ForwardNode,
    ) -> Result<String> {
        info!(
            "Creating forward: {} -> {}",
            local.as_protocol_string(),
            remote.as_protocol_string()
        );

        let cmd = format!(
            "fport {} {}",
            local.as_protocol_string(),
            remote.as_protocol_string()
        );
        self.send_command(&cmd).await?;

        let response = self.read_response_string().await?;
        debug!("Forward response: {}", response);
        Ok(response)
    }

    /// Create a reverse port forward (rport)
    ///
    /// Reserve remote traffic to local host.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::{HdcClient, ForwardNode};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// // Forward device TCP 8080 to local TCP 8081
    /// client.rport(ForwardNode::Tcp(8080), ForwardNode::Tcp(8081)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn rport(
        &mut self,
        remote: crate::forward::ForwardNode,
        local: crate::forward::ForwardNode,
    ) -> Result<String> {
        info!(
            "Creating reverse forward: {} -> {}",
            remote.as_protocol_string(),
            local.as_protocol_string()
        );

        let cmd = format!(
            "rport {} {}",
            remote.as_protocol_string(),
            local.as_protocol_string()
        );
        self.send_command(&cmd).await?;

        let response = self.read_response_string().await?;
        debug!("Reverse forward response: {}", response);
        Ok(response)
    }

    /// List all forward/reverse tasks
    ///
    /// Note: This command does not require a device connection.
    /// It lists forwards across all devices.
    pub async fn fport_list(&mut self) -> Result<Vec<String>> {
        info!("Listing forward tasks");

        // fport ls doesn't need connectKey, use a temporary connection
        let mut temp_client = Self::new(&self.address);
        temp_client.connect_internal().await?;

        temp_client.send_command("fport ls").await?;
        let response = temp_client.read_response_string().await?;
        debug!("Forward list response: {}", response);

        // Check for error messages
        if response.starts_with("[Fail]") {
            return Err(HdcError::Protocol(response));
        }

        // Parse the response - each line is a forward task
        let tasks: Vec<String> = response
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect();

        Ok(tasks)
    }

    /// Remove a forward/reverse task by task string
    ///
    /// Note: This command does not require a device connection.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// client.fport_remove("tcp:8080 tcp:8081").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fport_remove(&mut self, task_str: &str) -> Result<String> {
        info!("Removing forward task: {}", task_str);

        // fport rm doesn't need connectKey, use a temporary connection
        let mut temp_client = Self::new(&self.address);
        temp_client.connect_internal().await?;

        let cmd = format!("fport rm {}", task_str);
        temp_client.send_command(&cmd).await?;

        let response = temp_client.read_response_string().await?;
        debug!("Remove forward response: {}", response);

        // Check for error messages
        if response.starts_with("[Fail]") {
            return Err(HdcError::Protocol(response));
        }

        Ok(response)
    }

    // ========== App Commands ==========

    /// Install application package(s) to device
    ///
    /// # Arguments
    /// * `paths` - Single or multiple package paths (.hap, .hsp) or directories
    /// * `options` - Install options (replace, shared)
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::{HdcClient, InstallOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// let opts = InstallOptions::new().replace(true);
    /// client.install(&["app.hap"], opts).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn install(
        &mut self,
        paths: &[&str],
        options: crate::app::InstallOptions,
    ) -> Result<String> {
        info!("Installing app: {:?} with options: {:?}", paths, options);

        let flags = options.to_flags();
        let paths_str = paths.join(" ");

        let cmd = if flags.is_empty() {
            format!("install {}", paths_str)
        } else {
            format!("install {} {}", flags, paths_str)
        };

        self.send_command(&cmd).await?;

        // Install may take time and send multiple responses
        let mut output = String::new();
        loop {
            match timeout(Duration::from_secs(30), self.read_response_string()).await {
                Ok(Ok(resp)) => {
                    if resp.is_empty() {
                        break;
                    }
                    output.push_str(&resp);

                    // Check if installation completed
                    if resp.contains("Success")
                        || resp.contains("success")
                        || resp.contains("Fail")
                        || resp.contains("fail")
                    {
                        break;
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    warn!("Timeout waiting for install response");
                    break;
                }
            }
        }

        debug!("Install output: {} bytes", output.len());
        Ok(output)
    }

    /// Uninstall application package from device
    ///
    /// # Arguments
    /// * `package` - Package name to uninstall
    /// * `options` - Uninstall options (keep_data, shared)
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::{HdcClient, UninstallOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// let opts = UninstallOptions::new().keep_data(true);
    /// client.uninstall("com.example.app", opts).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn uninstall(
        &mut self,
        package: &str,
        options: crate::app::UninstallOptions,
    ) -> Result<String> {
        info!("Uninstalling app: {} with options: {:?}", package, options);

        let flags = options.to_flags();

        let cmd = if flags.is_empty() {
            format!("uninstall {}", package)
        } else {
            format!("uninstall {} {}", flags, package)
        };

        self.send_command(&cmd).await?;

        let response = self.read_response_string().await?;
        debug!("Uninstall response: {}", response);
        Ok(response)
    }

    /// Display device logs using hilog
    ///
    /// This method streams logs from the device. The log stream will continue until
    /// the connection is closed or an error occurs.
    ///
    /// # Arguments
    /// * `args` - Optional arguments for hilog command (e.g., "-h" for help, "-t app" for app logs)
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// # client.connect_device("device_id").await?;
    /// // Display all logs
    /// let logs = client.hilog(None).await?;
    /// println!("{}", logs);
    ///
    /// // Display only app logs
    /// let app_logs = client.hilog(Some("-t app")).await?;
    /// println!("{}", app_logs);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn hilog(&mut self, args: Option<&str>) -> Result<String> {
        info!("Reading hilog: {:?}", args);

        let cmd = if let Some(args) = args {
            format!("hilog {}", args)
        } else {
            "hilog".to_string()
        };

        self.send_command(&cmd).await?;

        let mut output = String::new();

        // Read log stream with extended timeout
        // Hilog streams continuously, we read for a reasonable amount of time
        loop {
            match timeout(Duration::from_secs(5), self.read_response_string()).await {
                Ok(Ok(resp)) => {
                    if resp.is_empty() {
                        break;
                    }
                    output.push_str(&resp);

                    // For continuous log streaming, check if user wants to stop
                    // In practice, you might want to use a callback or channel here
                    // to allow real-time log streaming instead of buffering
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    // Timeout - check if we got any data
                    if output.is_empty() {
                        warn!("Timeout waiting for hilog response");
                        return Err(HdcError::Timeout);
                    }
                    // Otherwise, this might just be the end of the log stream
                    break;
                }
            }
        }

        debug!("Hilog output: {} bytes", output.len());
        Ok(output)
    }

    /// Stream hilog output continuously with a callback
    ///
    /// This method streams logs from the device and calls the provided callback
    /// for each log chunk received. The stream continues until an error occurs
    /// or the callback returns false.
    ///
    /// # Arguments
    /// * `args` - Optional arguments for hilog command
    /// * `callback` - Function to call for each log chunk. Return false to stop streaming.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// # client.connect_device("device_id").await?;
    /// client.hilog_stream(None, |log_chunk| {
    ///     print!("{}", log_chunk);
    ///     true // Continue streaming
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn hilog_stream<F>(&mut self, args: Option<&str>, mut callback: F) -> Result<()>
    where
        F: FnMut(&str) -> bool,
    {
        info!("Starting hilog stream: {:?}", args);

        let cmd = if let Some(args) = args {
            format!("hilog {}", args)
        } else {
            "hilog".to_string()
        };

        self.send_command(&cmd).await?;

        // Stream logs continuously
        loop {
            match timeout(Duration::from_secs(30), self.read_response_string()).await {
                Ok(Ok(resp)) => {
                    if resp.is_empty() {
                        break;
                    }

                    // Call user callback with log chunk
                    if !callback(&resp) {
                        info!("Hilog stream stopped by callback");
                        break;
                    }
                }
                Ok(Err(e)) => {
                    warn!("Error reading hilog stream: {:?}", e);
                    return Err(e);
                }
                Err(_) => {
                    warn!("Timeout reading hilog stream");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Wait for any device to connect
    ///
    /// This command blocks until at least one device is connected.
    /// If a device is already connected, it returns immediately.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// // Wait for any device
    /// let device = client.wait_for_device().await?;
    /// println!("Device connected: {}", device);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait_for_device(&mut self) -> Result<String> {
        info!("Waiting for device...");

        self.send_command("wait").await?;

        let response = self.read_response_string().await?;
        debug!("Wait for device response: {}", response);

        // Response format: "Wait for connected target is <device_id>"
        if let Some(device_id) = response.split("is ").nth(1) {
            Ok(device_id.trim().to_string())
        } else {
            // Fallback: just return the whole response
            Ok(response.trim().to_string())
        }
    }

    /// Monitor device list changes with a callback
    ///
    /// This function continuously polls the device list and calls the callback
    /// when changes are detected. The polling interval can be configured.
    ///
    /// Note: HDC doesn't have a native "track-devices" command like adb,
    /// so this implementation uses polling to detect changes. Each poll creates
    /// a new connection to ensure reliability.
    ///
    /// # Arguments
    /// * `interval` - Polling interval (recommended: 1-3 seconds)
    /// * `callback` - Function called when device list changes. Return false to stop monitoring.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # use std::time::Duration;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// client.monitor_devices(Duration::from_secs(2), |devices| {
    ///     println!("Device list changed:");
    ///     for device in devices {
    ///         println!("  - {}", device);
    ///     }
    ///     true // Continue monitoring
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn monitor_devices<F>(&mut self, interval: Duration, mut callback: F) -> Result<()>
    where
        F: FnMut(&[String]) -> bool,
    {
        info!("Starting device monitoring with interval: {:?}", interval);

        let mut previous_devices: Vec<String> = Vec::new();

        loop {
            // Reconnect for each poll to ensure fresh connection
            // HDC server closes connection after each request
            if let Err(e) = self.connect_internal().await {
                warn!("Failed to reconnect during monitoring: {:?}", e);
                tokio::time::sleep(interval).await;
                continue;
            }

            // Get current device list
            match self.list_targets().await {
                Ok(devices) => {
                    // Check if device list has changed
                    if devices != previous_devices {
                        debug!(
                            "Device list changed: {:?} -> {:?}",
                            previous_devices, devices
                        );

                        // Call user callback
                        if !callback(&devices) {
                            info!("Device monitoring stopped by callback");
                            break;
                        }

                        previous_devices = devices;
                    }
                }
                Err(e) => {
                    warn!("Error listing devices during monitoring: {:?}", e);
                    // Continue monitoring even if there's an error
                }
            }

            // Wait before next poll
            tokio::time::sleep(interval).await;
        }

        Ok(())
    }

    /// Send file to device
    ///
    /// Transfer a file from local path to remote device path.
    ///
    /// # Arguments
    /// * `local_path` - Local file path to send
    /// * `remote_path` - Remote device path destination
    /// * `options` - File transfer options (timestamp, sync, compress, etc.)
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::{HdcClient, FileTransferOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// # client.connect_device("device_id").await?;
    /// let opts = FileTransferOptions::new()
    ///     .hold_timestamp(true)
    ///     .compress(true);
    /// client.file_send("test.txt", "/data/local/tmp/test.txt", opts).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn file_send(
        &mut self,
        local_path: &str,
        remote_path: &str,
        options: crate::file::FileTransferOptions,
    ) -> Result<String> {
        info!("Sending file: {} -> {}", local_path, remote_path);

        // Validate paths
        if !crate::file::validate_path(local_path) || !crate::file::validate_path(remote_path) {
            return Err(HdcError::Protocol("Invalid file path".to_string()));
        }

        // Build command
        let flags = options.to_flags();
        let cmd = if flags.is_empty() {
            format!("file send {} {}", local_path, remote_path)
        } else {
            format!("file send {} {} {}", flags, local_path, remote_path)
        };

        info!("File send command: {}", cmd);
        self.send_command(&cmd).await?;

        // Read transfer responses
        let mut output = String::new();
        loop {
            match timeout(Duration::from_secs(60), self.read_response_string()).await {
                Ok(Ok(resp)) => {
                    if resp.is_empty() {
                        break;
                    }
                    output.push_str(&resp);

                    // Check for completion indicators
                    if resp.contains("FileTransfer finish")
                        || resp.contains("Transfer finish")
                        || resp.contains("[Fail]")
                        || resp.contains("fail")
                    {
                        break;
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    warn!("Timeout during file transfer");
                    if output.is_empty() {
                        return Err(HdcError::Timeout);
                    }
                    break;
                }
            }
        }

        debug!("File send output: {} bytes", output.len());
        Ok(output)
    }

    /// Receive file from device
    ///
    /// Transfer a file from remote device path to local path.
    ///
    /// # Arguments
    /// * `remote_path` - Remote device file path to receive
    /// * `local_path` - Local destination path
    /// * `options` - File transfer options (timestamp, sync, compress, etc.)
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::{HdcClient, FileTransferOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// # client.connect_device("device_id").await?;
    /// let opts = FileTransferOptions::new().hold_timestamp(true);
    /// client.file_recv("/data/local/tmp/test.txt", "test.txt", opts).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn file_recv(
        &mut self,
        remote_path: &str,
        local_path: &str,
        options: crate::file::FileTransferOptions,
    ) -> Result<String> {
        info!("Receiving file: {} -> {}", remote_path, local_path);

        // Validate paths
        if !crate::file::validate_path(local_path) || !crate::file::validate_path(remote_path) {
            return Err(HdcError::Protocol("Invalid file path".to_string()));
        }

        // Build command
        let flags = options.to_flags();
        let cmd = if flags.is_empty() {
            format!("file recv {} {}", remote_path, local_path)
        } else {
            format!("file recv {} {} {}", flags, remote_path, local_path)
        };

        info!("File recv command: {}", cmd);
        self.send_command(&cmd).await?;

        // Read transfer responses
        let mut output = String::new();
        loop {
            match timeout(Duration::from_secs(60), self.read_response_string()).await {
                Ok(Ok(resp)) => {
                    if resp.is_empty() {
                        break;
                    }
                    output.push_str(&resp);

                    // Check for completion indicators
                    if resp.contains("FileTransfer finish")
                        || resp.contains("Transfer finish")
                        || resp.contains("[Fail]")
                        || resp.contains("fail")
                    {
                        break;
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    warn!("Timeout during file transfer");
                    if output.is_empty() {
                        return Err(HdcError::Timeout);
                    }
                    break;
                }
            }
        }

        debug!("File recv output: {} bytes", output.len());
        Ok(output)
    }
}

impl Drop for HdcClient {
    fn drop(&mut self) {
        if self.stream.is_some() {
            debug!("HdcClient dropped, connection will be closed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = HdcClient::new("127.0.0.1:8710");
        assert_eq!(client.address, "127.0.0.1:8710");
        assert!(!client.is_connected());
    }
}
