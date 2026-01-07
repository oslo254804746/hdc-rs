//! Channel handshake protocol

use tracing::debug;

use super::HANDSHAKE_BANNER;
use crate::error::{HdcError, Result};

/// Channel handshake structure
///
/// This is exchanged between client and server during initial connection.
/// Layout matches the C++ struct in `src/common/channel.h`:
/// ```c++
/// struct ChannelHandShake {
///     char banner[12];
///     union {
///         uint32_t channelId;
///         char connectKey[32];
///     };
///     char version[64];
/// }
/// ```
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ChannelHandShake {
    /// Banner: "OHOS HDC" + feature flags (12 bytes)
    pub banner: [u8; 12],
    /// Channel ID (received from server) or connect key (sent to server)
    pub channel_id_or_key: [u8; 32],
    /// Version string (64 bytes)
    pub version: [u8; 64],
}

impl ChannelHandShake {
    /// Size of the full handshake structure in bytes (with version)
    pub const SIZE: usize = 12 + 32 + 64;

    /// Size of the handshake without version field
    pub const SIZE_WITHOUT_VERSION: usize = 12 + 32;

    /// Offset of feature tag in banner
    const BANNER_FEATURE_TAG_OFFSET: usize = 7;

    /// Tag indicating huge buffer support
    const HUGE_BUF_TAG: u8 = b'1';

    /// Create a new handshake from raw bytes
    ///
    /// Supports two formats:
    /// - 44 bytes: banner + channel_id_or_key (without version)
    /// - 108 bytes: banner + channel_id_or_key + version (full)
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        // HDC server may send handshake with or without version field
        // 44 bytes = 12 (banner) + 32 (channel_id_or_key)
        // 108 bytes = 12 (banner) + 32 (channel_id_or_key) + 64 (version)
        if data.len() < Self::SIZE_WITHOUT_VERSION {
            return Err(HdcError::Protocol(format!(
                "Handshake data too short: expected at least {}, got {}",
                Self::SIZE_WITHOUT_VERSION,
                data.len()
            )));
        }

        let mut handshake = ChannelHandShake {
            banner: [0; 12],
            channel_id_or_key: [0; 32],
            version: [0; 64],
        };

        handshake.banner.copy_from_slice(&data[0..12]);
        handshake.channel_id_or_key.copy_from_slice(&data[12..44]);

        // Version field is optional
        if data.len() >= Self::SIZE {
            handshake.version.copy_from_slice(&data[44..108]);
        } else {
            debug!(
                "Received handshake without version field ({} bytes)",
                data.len()
            );
        }

        Ok(handshake)
    }

    /// Convert handshake to bytes (full format with version)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.extend_from_slice(&self.banner);
        bytes.extend_from_slice(&self.channel_id_or_key);
        bytes.extend_from_slice(&self.version);
        bytes
    }

    /// Convert handshake to bytes without version field
    pub fn to_bytes_without_version(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE_WITHOUT_VERSION);
        bytes.extend_from_slice(&self.banner);
        bytes.extend_from_slice(&self.channel_id_or_key);
        bytes
    }

    /// Verify the banner is valid
    pub fn verify_banner(&self) -> Result<()> {
        if !self.banner.starts_with(HANDSHAKE_BANNER) {
            return Err(HdcError::InvalidBanner(self.banner.to_vec()));
        }
        Ok(())
    }

    /// Get channel ID from the handshake (server -> client)
    pub fn get_channel_id(&self) -> u32 {
        // Channel ID is stored in first 4 bytes of channel_id_or_key, network byte order
        u32::from_be_bytes([
            self.channel_id_or_key[0],
            self.channel_id_or_key[1],
            self.channel_id_or_key[2],
            self.channel_id_or_key[3],
        ])
    }

    /// Set channel ID in the handshake
    pub fn set_channel_id(&mut self, channel_id: u32) {
        let bytes = channel_id.to_be_bytes();
        self.channel_id_or_key[0..4].copy_from_slice(&bytes);
    }

    /// Set connect key in the handshake (client -> server)
    pub fn set_connect_key(&mut self, connect_key: &str) {
        self.channel_id_or_key.fill(0);
        let key_bytes = connect_key.as_bytes();
        let copy_len = key_bytes.len().min(32);
        self.channel_id_or_key[0..copy_len].copy_from_slice(&key_bytes[0..copy_len]);
    }

    /// Get connect key from the handshake
    pub fn get_connect_key(&self) -> String {
        // Find null terminator
        let end = self
            .channel_id_or_key
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(32);
        String::from_utf8_lossy(&self.channel_id_or_key[0..end]).to_string()
    }

    /// Check if server supports stable buffer mode
    pub fn is_stable_buf(&self) -> bool {
        if self.banner.len() > Self::BANNER_FEATURE_TAG_OFFSET {
            self.banner[Self::BANNER_FEATURE_TAG_OFFSET] != Self::HUGE_BUF_TAG
        } else {
            true
        }
    }

    /// Get version string
    pub fn get_version(&self) -> String {
        let end = self.version.iter().position(|&b| b == 0).unwrap_or(64);
        String::from_utf8_lossy(&self.version[0..end]).to_string()
    }

    /// Set version string
    pub fn set_version(&mut self, version: &str) {
        self.version.fill(0);
        let version_bytes = version.as_bytes();
        let copy_len = version_bytes.len().min(64);
        self.version[0..copy_len].copy_from_slice(&version_bytes[0..copy_len]);
    }
}

impl Default for ChannelHandShake {
    fn default() -> Self {
        Self {
            banner: [0; 12],
            channel_id_or_key: [0; 32],
            version: [0; 64],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handshake_size() {
        assert_eq!(ChannelHandShake::SIZE, 108);
        assert_eq!(ChannelHandShake::SIZE_WITHOUT_VERSION, 44);
        assert_eq!(std::mem::size_of::<ChannelHandShake>(), 108);
    }

    #[test]
    fn test_handshake_from_bytes_short() {
        // Test 44-byte handshake (without version)
        let mut data = vec![0u8; 44];
        data[..8].copy_from_slice(b"OHOS HDC");
        data[12..16].copy_from_slice(&0x12345678u32.to_be_bytes());

        let hs = ChannelHandShake::from_bytes(&data).unwrap();
        assert_eq!(hs.get_channel_id(), 0x12345678);
        assert_eq!(hs.get_version(), "");
    }

    #[test]
    fn test_handshake_from_bytes_full() {
        // Test 108-byte handshake (with version)
        let mut data = vec![0u8; 108];
        data[..8].copy_from_slice(b"OHOS HDC");
        data[12..16].copy_from_slice(&0x12345678u32.to_be_bytes());
        data[44..49].copy_from_slice(b"3.2.0");

        let hs = ChannelHandShake::from_bytes(&data).unwrap();
        assert_eq!(hs.get_channel_id(), 0x12345678);
        assert_eq!(hs.get_version(), "3.2.0");
    }

    #[test]
    fn test_channel_id() {
        let mut hs = ChannelHandShake::default();
        hs.set_channel_id(0x12345678);
        assert_eq!(hs.get_channel_id(), 0x12345678);
    }

    #[test]
    fn test_connect_key() {
        let mut hs = ChannelHandShake::default();
        hs.set_connect_key("test-device-001");
        assert_eq!(hs.get_connect_key(), "test-device-001");
    }

    #[test]
    fn test_version() {
        let mut hs = ChannelHandShake::default();
        hs.set_version("3.2.0");
        assert_eq!(hs.get_version(), "3.2.0");
    }

    #[test]
    fn test_to_bytes_without_version() {
        let mut hs = ChannelHandShake::default();
        hs.banner[..8].copy_from_slice(b"OHOS HDC");
        hs.set_channel_id(0x12345678);

        let bytes = hs.to_bytes_without_version();
        assert_eq!(bytes.len(), 44);
        assert_eq!(&bytes[..8], b"OHOS HDC");
    }
}
