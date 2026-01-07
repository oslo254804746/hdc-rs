//! HDC protocol implementation

pub mod channel;
pub mod command;
pub mod packet;

pub use channel::ChannelHandShake;
pub use command::HdcCommand;
pub use packet::PacketCodec;

/// HDC handshake banner
pub const HANDSHAKE_BANNER: &[u8] = b"OHOS HDC";

/// Maximum packet size (511KB for large transfers)
pub const MAX_PACKET_SIZE: usize = 511 * 1024;

/// Default buffer size
pub const DEFAULT_BUF_SIZE: usize = 1024;

/// Size of packet length prefix (4 bytes, big-endian)
pub const PACKET_LENGTH_SIZE: usize = 4;
