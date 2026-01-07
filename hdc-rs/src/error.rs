//! Error types for HDC client operations

use std::io;
use thiserror::Error;

/// Result type alias for HDC operations
pub type Result<T> = std::result::Result<T, HdcError>;

/// Errors that can occur during HDC operations
#[derive(Error, Debug)]
pub enum HdcError {
    /// I/O error occurred during communication
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Invalid protocol data received
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Handshake failed
    #[error("Handshake failed: {0}")]
    HandshakeFailed(String),

    /// Connection not established
    #[error("Not connected to HDC server")]
    NotConnected,

    /// Invalid banner received
    #[error("Invalid banner: expected 'OHOS HDC', got {0:?}")]
    InvalidBanner(Vec<u8>),

    /// Buffer size error
    #[error("Buffer error: {0}")]
    BufferError(String),

    /// Command execution failed
    #[error("Command failed: {0}")]
    CommandFailed(String),

    /// Timeout occurred
    #[error("Operation timed out")]
    Timeout,

    /// Device not found
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    /// UTF-8 conversion error
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}
