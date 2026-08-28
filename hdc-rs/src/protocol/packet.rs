//! Packet encoding and decoding

use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, trace};

use super::{MAX_PACKET_SIZE, PACKET_LENGTH_SIZE};
use crate::error::{HdcError, Result};

/// Codec for HDC packet protocol
///
/// HDC uses a simple length-prefixed protocol:
/// ```text
/// +------------------+
/// | 4 bytes: length  |  (big-endian u32)
/// +------------------+
/// | N bytes: data    |
/// +------------------+
/// ```
pub struct PacketCodec {
    #[allow(dead_code)]
    read_buf: BytesMut,
}

impl PacketCodec {
    /// Create a new packet codec
    pub fn new() -> Self {
        Self {
            read_buf: BytesMut::with_capacity(MAX_PACKET_SIZE),
        }
    }

    /// Encode data into a packet with length prefix
    pub fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() > MAX_PACKET_SIZE {
            return Err(HdcError::BufferError(format!(
                "Data size {} exceeds maximum packet size {}",
                data.len(),
                MAX_PACKET_SIZE
            )));
        }

        let size = data.len() as u32;
        let mut buf = Vec::with_capacity(PACKET_LENGTH_SIZE + data.len());

        // Write length as big-endian
        buf.put_u32(size);
        buf.extend_from_slice(data);

        trace!("Encoded packet: size={}, data_len={}", size, data.len());
        Ok(buf)
    }

    /// Read and decode a packet from a stream
    pub async fn decode<S>(&mut self, stream: &mut S) -> Result<Vec<u8>>
    where
        S: AsyncReadExt + Unpin,
    {
        // Read the first length byte separately so a clean transport close is
        // distinguishable from a truncated length prefix.
        let mut len_buf = [0u8; PACKET_LENGTH_SIZE];
        match stream.read_exact(&mut len_buf[..1]).await {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(HdcError::ConnectionClosed);
            }
            Err(error) => return Err(HdcError::Io(error)),
        }
        stream.read_exact(&mut len_buf[1..]).await?;
        let packet_len = u32::from_be_bytes(len_buf) as usize;

        if packet_len == 0 {
            // Empty packet - return empty vec instead of error
            debug!("Received zero-length packet");
            return Ok(Vec::new());
        }

        if packet_len > MAX_PACKET_SIZE {
            return Err(HdcError::Protocol(format!(
                "Packet size {} exceeds maximum {}",
                packet_len, MAX_PACKET_SIZE
            )));
        }

        // Read packet data
        let mut data = vec![0u8; packet_len];
        stream.read_exact(&mut data).await?;

        debug!("Decoded packet: size={}", packet_len);
        Ok(data)
    }

    /// Write an encoded packet to a stream
    pub async fn write_packet<S>(&self, stream: &mut S, data: &[u8]) -> Result<()>
    where
        S: AsyncWriteExt + Unpin,
    {
        let packet = self.encode(data)?;
        stream.write_all(&packet).await?;
        stream.flush().await?;
        debug!(
            "Wrote packet: {} bytes (data: {} bytes)",
            packet.len(),
            data.len()
        );
        Ok(())
    }

    /// Read a raw packet (already has length prefix)
    pub async fn read_packet<S>(&mut self, stream: &mut S) -> Result<Vec<u8>>
    where
        S: AsyncReadExt + Unpin,
    {
        self.decode(stream).await
    }
}

impl Default for PacketCodec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{duplex, AsyncWriteExt};

    #[test]
    fn test_encode() {
        let codec = PacketCodec::new();
        let data = b"Hello, HDC!";
        let packet = codec.encode(data).unwrap();

        // Check length prefix
        assert_eq!(packet.len(), 4 + data.len());
        let len = u32::from_be_bytes([packet[0], packet[1], packet[2], packet[3]]);
        assert_eq!(len as usize, data.len());

        // Check data
        assert_eq!(&packet[4..], data);
    }

    #[test]
    fn test_encode_empty() {
        let codec = PacketCodec::new();
        let data = b"";
        let packet = codec.encode(data).unwrap();
        assert_eq!(packet.len(), 4);
        let len = u32::from_be_bytes([packet[0], packet[1], packet[2], packet[3]]);
        assert_eq!(len, 0);
    }

    #[tokio::test]
    async fn clean_transport_close_is_distinct_from_truncation() {
        let (mut reader, writer) = duplex(64);
        drop(writer);

        let error = PacketCodec::new().decode(&mut reader).await.unwrap_err();
        assert!(matches!(error, HdcError::ConnectionClosed));
    }

    #[tokio::test]
    async fn truncated_length_prefix_is_an_io_error() {
        let (mut reader, mut writer) = duplex(64);
        writer.write_all(&[0, 0]).await.unwrap();
        drop(writer);

        let error = PacketCodec::new().decode(&mut reader).await.unwrap_err();
        assert!(
            matches!(error, HdcError::Io(error) if error.kind() == std::io::ErrorKind::UnexpectedEof)
        );
    }

    #[tokio::test]
    async fn truncated_packet_body_is_an_io_error() {
        let (mut reader, mut writer) = duplex(64);
        writer.write_all(&[0, 0, 0, 5, 1, 2]).await.unwrap();
        drop(writer);

        let error = PacketCodec::new().decode(&mut reader).await.unwrap_err();
        assert!(
            matches!(error, HdcError::Io(error) if error.kind() == std::io::ErrorKind::UnexpectedEof)
        );
    }
}
