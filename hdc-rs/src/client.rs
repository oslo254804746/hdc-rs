//! HDC client implementation

use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::error::{HdcError, Result};
use crate::protocol::{ChannelHandShake, HdcCommand, PacketCodec};

/// Default connection timeout
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

fn parse_jpid_response(response: &str) -> Vec<String> {
    response
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
enum ResponseFrame {
    Data(Vec<u8>),
    ChannelClose,
}

fn decode_response_frame(data: Vec<u8>) -> ResponseFrame {
    if data.len() >= 2 {
        let cmd_code = u16::from_le_bytes([data[0], data[1]]);
        if let Some(command) = HdcCommand::from_u16(cmd_code) {
            if command == HdcCommand::KernelChannelClose {
                debug!("Received channel-close frame");
                return ResponseFrame::ChannelClose;
            }
            if command.is_response() {
                debug!("Response has command prefix: {:?}", command);
                return ResponseFrame::Data(data[2..].to_vec());
            }
        }
    }

    ResponseFrame::Data(data)
}

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

    /// Drop a channel after commands that consume their connection.
    fn invalidate_connection(&mut self) {
        self.stream = None;
        self.handshake_ok = false;
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

    async fn read_response_frame(&mut self) -> Result<ResponseFrame> {
        Ok(decode_response_frame(self.read_response().await?))
    }

    /// Read raw response frames until the transport reaches EOF.
    ///
    /// Empty frames are ignored. A clean transport close ends the read
    /// normally; framing errors (including a truncated packet), other I/O
    /// errors, and timeouts are returned to the caller, which must invalidate
    /// the channel before returning to its user.
    async fn read_raw_until_eof(&mut self, frame_timeout: Duration) -> Result<Vec<u8>> {
        let mut output = Vec::new();

        loop {
            match timeout(frame_timeout, self.read_response()).await {
                Ok(Ok(data)) => output.extend_from_slice(&data),
                Ok(Err(HdcError::ConnectionClosed)) => break,
                Ok(Err(error)) => return Err(error),
                Err(_) => return Err(HdcError::Timeout),
            }
        }

        Ok(output)
    }

    /// Read a task response until the server closes the channel.
    async fn read_until_channel_end(&mut self, frame_timeout: Duration) -> Result<String> {
        let mut output = Vec::new();

        loop {
            let frame = match timeout(frame_timeout, self.read_response_frame()).await {
                Ok(Ok(frame)) => frame,
                Ok(Err(HdcError::ConnectionClosed)) => break,
                Ok(Err(error)) => return Err(error),
                Err(_) => return Err(HdcError::Timeout),
            };

            match frame {
                ResponseFrame::ChannelClose => break,
                ResponseFrame::Data(data) => output.extend_from_slice(&data),
            }
        }

        Ok(String::from_utf8(output)?)
    }

    async fn run_terminal_command(
        &mut self,
        command: &str,
        frame_timeout: Duration,
    ) -> Result<String> {
        let result = async {
            self.send_command(command).await?;
            self.read_until_channel_end(frame_timeout).await
        }
        .await;
        self.invalidate_connection();
        result
    }

    /// Read response as string
    pub async fn read_response_string(&mut self) -> Result<String> {
        let frame = match self.read_response_frame().await {
            Ok(frame) => frame,
            Err(error) => {
                self.invalidate_connection();
                return Err(error);
            }
        };

        match frame {
            ResponseFrame::ChannelClose => {
                self.invalidate_connection();
                Ok(String::new())
            }
            ResponseFrame::Data(data) => Ok(String::from_utf8(data)?),
        }
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

        let result: Result<String> = async {
            self.send_command(&full_cmd).await?;

            let data = self.read_raw_until_eof(Duration::from_secs(5)).await?;
            debug!("Shell response: {} bytes", data.len());
            Ok(String::from_utf8_lossy(&data).to_string())
        }
        .await;

        self.invalidate_connection();
        let output = result?;

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

    /// List connected devices/targets.
    ///
    /// This one-shot task drains its response and disconnects the client before
    /// returning. Create a fresh client before issuing another terminal task.
    pub async fn list_targets(&mut self) -> Result<Vec<String>> {
        info!("Listing targets");

        let response = self
            .run_terminal_command("list targets", Duration::from_secs(30))
            .await?;
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

    /// List connected targets with verbose upstream output.
    ///
    /// This one-shot task drains its response and disconnects the client before
    /// returning. Create a fresh client before issuing another terminal task.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// let details = client.list_targets_verbose().await?;
    /// println!("{}", details);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_targets_verbose(&mut self) -> Result<String> {
        self.run_terminal_command("list targets -v", Duration::from_secs(30))
            .await
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

    /// Check server version.
    ///
    /// This one-shot task drains its response and disconnects the client before
    /// returning. Create a fresh client before issuing another terminal task.
    pub async fn check_server(&mut self) -> Result<String> {
        info!("Checking server version");

        let response = self
            .run_terminal_command("checkserver", Duration::from_secs(30))
            .await?;

        debug!("Server version: {}", response);
        Ok(response)
    }

    /// Get HDC server/client protocol version information.
    ///
    /// This one-shot task drains its response and disconnects the client before
    /// returning. Create a fresh client before issuing another terminal task.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// let version = client.version().await?;
    /// println!("{}", version);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn version(&mut self) -> Result<String> {
        self.run_terminal_command("version", Duration::from_secs(30))
            .await
    }

    /// Get HDC help text.
    ///
    /// This one-shot task drains its response and disconnects the client before
    /// returning. Create a fresh client before issuing another terminal task.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// let help = client.help(false).await?;
    /// println!("{}", help);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn help(&mut self, verbose: bool) -> Result<String> {
        let cmd = if verbose { "help verbose" } else { "help" };
        self.run_terminal_command(cmd, Duration::from_secs(30))
            .await
    }

    /// Ask the HDC server to discover targets.
    ///
    /// This one-shot task drains its response and disconnects the client before
    /// returning. Create a fresh client before issuing another terminal task.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// let result = client.discover().await?;
    /// println!("{}", result);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover(&mut self) -> Result<String> {
        self.run_terminal_command("discover", Duration::from_secs(30))
            .await
    }

    /// Check target device state.
    ///
    /// This one-shot task drains its response and disconnects the client before
    /// returning. Create a fresh client before issuing another terminal task.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// let state = client.check_device(None).await?;
    /// println!("{}", state);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn check_device(&mut self, connect_key: Option<&str>) -> Result<String> {
        let cmd = match connect_key {
            Some(key) if !key.is_empty() => format!("checkdevice {}", key),
            _ => "checkdevice".to_string(),
        };
        self.run_terminal_command(&cmd, Duration::from_secs(30))
            .await
    }

    /// Connect to a target by connect key, such as `host:port`.
    ///
    /// The task consumes its channel; the client is disconnected when this
    /// method returns.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// client.target_connect("192.168.0.2:10178").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn target_connect(&mut self, key: &str) -> Result<String> {
        let cmd = crate::command_builder::target_connect(key, false);
        self.run_terminal_command(&cmd, Duration::from_secs(30))
            .await
    }

    /// Disconnect a target by connect key.
    ///
    /// The task consumes its channel; the client is disconnected when this
    /// method returns.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// client.target_disconnect("192.168.0.2:10178").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn target_disconnect(&mut self, key: &str) -> Result<String> {
        let cmd = crate::command_builder::target_connect(key, true);
        self.run_terminal_command(&cmd, Duration::from_secs(30))
            .await
    }

    /// Select any available target.
    ///
    /// The task consumes its channel; the client is disconnected when this
    /// method returns.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// client.connect_any().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_any(&mut self) -> Result<String> {
        self.run_terminal_command("any", Duration::from_secs(30))
            .await
    }

    /// Reconnect the current or specified target.
    ///
    /// The task consumes its channel; the client is disconnected when this
    /// method returns.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// client.reconnect_target(None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reconnect_target(&mut self, connect_key: Option<&str>) -> Result<String> {
        let cmd = crate::command_builder::reconnect_target(connect_key);
        self.run_terminal_command(&cmd, Duration::from_secs(30))
            .await
    }

    /// Mount the target filesystem.
    ///
    /// The task consumes its channel; the client is disconnected when this
    /// method returns.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::HdcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// client.target_mount().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn target_mount(&mut self) -> Result<String> {
        let cmd = crate::command_builder::target_mount();
        self.run_terminal_command(&cmd, Duration::from_secs(30))
            .await
    }

    /// Boot the target, optionally using an upstream boot mode.
    ///
    /// The task consumes its channel; the client is disconnected when this
    /// method returns.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::{HdcClient, TargetBootMode};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// client.target_boot(Some(TargetBootMode::Recovery)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn target_boot(
        &mut self,
        mode: Option<crate::device::TargetBootMode>,
    ) -> Result<String> {
        let cmd = crate::command_builder::target_boot(mode.as_ref().map(|mode| mode.as_arg()));
        self.run_terminal_command(&cmd, Duration::from_secs(30))
            .await
    }

    /// Switch daemon privilege mode. `true` renders `smode`, `false` renders `smode -r`.
    /// The client is disconnected when this task completes.
    pub async fn smode(&mut self, enable_root: bool) -> Result<String> {
        let cmd = crate::command_builder::smode(enable_root);
        self.run_terminal_command(&cmd, Duration::from_secs(30))
            .await
    }

    /// Switch daemon transport mode.
    /// The client is disconnected when this task completes.
    ///
    /// # Example
    /// ```no_run
    /// # use hdc_rs::{HdcClient, TargetMode};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = HdcClient::connect("127.0.0.1:8710").await?;
    /// client.tmode(TargetMode::Port(Some(10178))).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn tmode(&mut self, mode: crate::device::TargetMode) -> Result<String> {
        let cmd = match mode {
            crate::device::TargetMode::Usb => crate::command_builder::tmode_usb(),
            crate::device::TargetMode::Port(port) => crate::command_builder::tmode_port(port),
            crate::device::TargetMode::PortClose => crate::command_builder::tmode_port_close(),
        };
        self.run_terminal_command(&cmd, Duration::from_secs(30))
            .await
    }

    /// Execute a command on a specific device
    ///
    /// This is a convenience method that:
    /// 1. Connects to the specified device (re-handshake with connectKey)
    /// 2. Executes the command
    ///
    /// Note: This changes the client's current device setting.
    /// The task consumes its channel; the client is disconnected when this
    /// method returns.
    pub async fn target_command(&mut self, device_id: &str, cmd: &str) -> Result<String> {
        info!("Executing target command on {}: {}", device_id, cmd);

        let result = async {
            // Connect to device first (sets connectKey in handshake)
            self.connect_device(device_id).await?;
            self.run_terminal_command(cmd, Duration::from_secs(30))
                .await
        }
        .await;
        self.invalidate_connection();
        result
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
    /// The task drains its channel and disconnects the client when it returns.
    /// Create a fresh client before issuing another terminal task.
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
        let result = async {
            self.send_command(&cmd).await?;
            self.read_until_channel_end(Duration::from_secs(30)).await
        }
        .await;
        self.invalidate_connection();
        let response = result?;
        debug!("Forward response: {}", response);
        Ok(response)
    }

    /// Create a reverse port forward (rport)
    ///
    /// Reserve remote traffic to local host.
    /// The task drains its channel and disconnects the client when it returns.
    /// Create a fresh client before issuing another terminal task.
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
        let result = async {
            self.send_command(&cmd).await?;
            self.read_until_channel_end(Duration::from_secs(30)).await
        }
        .await;
        self.invalidate_connection();
        let response = result?;
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
    /// The install task consumes its channel; the client is disconnected when
    /// this method returns.
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

        options.validate()?;
        let flags = options.to_flags();
        let mut rendered_paths = Vec::with_capacity(paths.len());
        for path in paths {
            crate::app::validate_path_argument("install source path", path)?;
            rendered_paths.push(crate::app::render_hdc_argument(path));
        }
        let paths_str = rendered_paths.join(" ");

        let cmd = if flags.is_empty() {
            format!("install {}", paths_str)
        } else {
            format!("install {} {}", flags, paths_str)
        };

        self.run_terminal_command(&cmd, Duration::from_secs(30))
            .await
    }

    /// Uninstall application package from device
    ///
    /// # Arguments
    /// * `package` - Package name to uninstall
    /// * `options` - Uninstall options (keep_data, shared)
    ///
    /// The package is encoded as the upstream `-n` option. The task consumes
    /// its channel; the client is disconnected when this method returns.
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

        crate::app::validate_package_name(package)?;
        options.validate()?;
        let flags = options.to_flags();
        let package_arg = format!("\"-n {package}\"");

        let cmd = if flags.is_empty() {
            format!("uninstall {package_arg}")
        } else {
            format!("uninstall {} {}", flags, package_arg)
        };

        self.run_terminal_command(&cmd, Duration::from_secs(30))
            .await
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

        let result = async {
            self.send_command(&cmd).await?;

            let mut output = String::new();

            // Read log stream with extended timeout
            // Hilog streams continuously, we read for a reasonable amount of time
            loop {
                match timeout(Duration::from_secs(5), self.read_response()).await {
                    Ok(Ok(data)) if data.is_empty() => continue,
                    Ok(Ok(data)) => {
                        let resp = String::from_utf8(data)?;
                        output.push_str(&resp);

                        // For continuous log streaming, check if user wants to stop
                        // In practice, you might want to use a callback or channel here
                        // to allow real-time log streaming instead of buffering
                    }
                    Ok(Err(HdcError::ConnectionClosed)) => break,
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
        .await;
        self.invalidate_connection();
        result
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

        let result = async {
            self.send_command(&cmd).await?;

            // Stream logs continuously
            loop {
                match timeout(Duration::from_secs(30), self.read_response()).await {
                    Ok(Ok(data)) if data.is_empty() => continue,
                    Ok(Ok(data)) => {
                        let response = String::from_utf8(data)?;

                        // Call user callback with log chunk
                        if !callback(&response) {
                            info!("Hilog stream stopped by callback");
                            break;
                        }
                    }
                    Ok(Err(HdcError::ConnectionClosed)) => break,
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
        .await;
        self.invalidate_connection();
        result
    }

    /// List debug/JDWP process identifiers.
    ///
    /// The task consumes its channel; the client is disconnected when this
    /// method returns.
    pub async fn jpid(&mut self) -> Result<Vec<String>> {
        let response = self
            .run_terminal_command("jpid", Duration::from_secs(30))
            .await?;
        Ok(parse_jpid_response(&response))
    }

    /// Track debug/JDWP process changes.
    ///
    /// The client is disconnected when the callback stops or the stream ends.
    pub async fn track_jpid<F>(
        &mut self,
        include_release: bool,
        pid_only: bool,
        mut callback: F,
    ) -> Result<()>
    where
        F: FnMut(&str) -> bool,
    {
        let cmd = if pid_only {
            "track-jpid -p"
        } else if include_release {
            "track-jpid -a"
        } else {
            "track-jpid"
        };
        let result = async {
            self.send_command(cmd).await?;

            loop {
                match self.read_response().await {
                    Ok(data) if data.is_empty() => continue,
                    Ok(data) => {
                        let response = String::from_utf8(data)?;
                        if !callback(&response) {
                            break;
                        }
                    }
                    Err(HdcError::ConnectionClosed) => break,
                    Err(error) => return Err(error),
                }
            }

            Ok(())
        }
        .await;
        self.invalidate_connection();
        result
    }

    /// Wait for any device to connect
    ///
    /// This command blocks until at least one device is connected.
    /// If a device is already connected, it returns immediately.
    /// The upstream server reports no-device failures on the same channel;
    /// this method retries indefinitely until a device is found. On success or
    /// error, the client is disconnected before returning.
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

        loop {
            if let Err(error) = self.send_command("wait").await {
                self.invalidate_connection();
                return Err(error);
            }

            let response = match self.read_response_frame().await {
                Ok(ResponseFrame::Data(data)) => match String::from_utf8(data) {
                    Ok(response) => response,
                    Err(error) => {
                        self.invalidate_connection();
                        return Err(error.into());
                    }
                },
                Ok(ResponseFrame::ChannelClose) => {
                    self.invalidate_connection();
                    return Err(HdcError::ConnectionClosed);
                }
                Err(error) => {
                    self.invalidate_connection();
                    return Err(error);
                }
            };

            let response = response.trim().to_string();
            debug!("Wait for device response: {}", response);

            // The upstream server keeps this channel open and reports this
            // response until a target becomes available.
            if response == "[Fail]No any connected target" {
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }

            // Response format: "Wait for connected target is <device_id>".
            if let Some(device_id) = response
                .strip_prefix("Wait for connected target is ")
                .map(str::trim)
                .filter(|device_id| !device_id.is_empty())
            {
                self.invalidate_connection();
                return Ok(device_id.to_string());
            }

            self.invalidate_connection();
            return Err(HdcError::Protocol(response));
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
    /// The transfer task consumes its channel; the client is disconnected when
    /// this method returns.
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

        if !crate::file::validate_path(local_path) || !crate::file::validate_path(remote_path) {
            return Err(HdcError::Protocol("Invalid file path".to_string()));
        }
        crate::app::validate_path_argument("local path", local_path)?;
        crate::app::validate_path_argument("remote path", remote_path)?;

        options.validate()?;

        // Build command
        let flags = options.to_flags();
        let local_arg = crate::app::render_hdc_argument(local_path);
        let remote_arg = crate::app::render_hdc_argument(remote_path);
        let cmd = if flags.is_empty() {
            format!("file send {} {}", local_arg, remote_arg)
        } else {
            format!("file send {} {} {}", flags, local_arg, remote_arg)
        };

        info!("File send command: {}", cmd);
        self.run_terminal_command(&cmd, Duration::from_secs(60))
            .await
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
    /// The transfer task consumes its channel; the client is disconnected when
    /// this method returns.
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

        if !crate::file::validate_path(local_path) || !crate::file::validate_path(remote_path) {
            return Err(HdcError::Protocol("Invalid file path".to_string()));
        }
        crate::app::validate_path_argument("local path", local_path)?;
        crate::app::validate_path_argument("remote path", remote_path)?;

        options.validate()?;

        // Build command
        let flags = options.to_flags();
        let remote_arg = crate::app::render_hdc_argument(remote_path);
        let local_arg = crate::app::render_hdc_argument(local_path);
        let cmd = if flags.is_empty() {
            format!("file recv {} {}", remote_arg, local_arg)
        } else {
            format!("file recv {} {} {}", flags, remote_arg, local_arg)
        };

        info!("File recv command: {}", cmd);
        self.run_terminal_command(&cmd, Duration::from_secs(60))
            .await
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

    #[test]
    fn parses_jpid_response_lines() {
        let pids = parse_jpid_response(" 123 \n\n456\n\t789\t\n");
        assert_eq!(pids, vec!["123", "456", "789"]);
    }

    #[test]
    fn decodes_channel_close_without_exposing_payload() {
        assert_eq!(
            decode_response_frame(vec![2, 0, b'c', b'l', b'o', b's', b'e']),
            ResponseFrame::ChannelClose
        );
    }

    #[test]
    fn decodes_prefixed_response_payload() {
        assert_eq!(
            decode_response_frame(vec![9, 0, b'o', b'k']),
            ResponseFrame::Data(vec![b'o', b'k'])
        );
        assert_eq!(
            decode_response_frame(vec![0xE9, 0x03, b'o', b'k']),
            ResponseFrame::Data(vec![0xE9, 0x03, b'o', b'k'])
        );
        assert_eq!(
            decode_response_frame(vec![13, 0, b'1', b'.', b'2']),
            ResponseFrame::Data(vec![b'1', b'.', b'2'])
        );
        assert_eq!(
            decode_response_frame(vec![14, 0, b'o', b'k']),
            ResponseFrame::Data(vec![14, 0, b'o', b'k'])
        );
    }
}
