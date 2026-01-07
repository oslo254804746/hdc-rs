//! File transfer types and options for HDC

/// File transfer options for send/recv operations
#[derive(Debug, Clone, Default)]
pub struct FileTransferOptions {
    /// Hold target file timestamp (-a)
    hold_timestamp: bool,
    /// Sync mode: only update newer files (-sync)
    sync_mode: bool,
    /// Compress transfer (-z)
    /// Note: May not improve efficiency for already compressed files
    compress: bool,
    /// Mode sync (-m)
    mode_sync: bool,
    /// Send/receive file to debug application directory (-b)
    debug_dir: bool,
}

impl FileTransferOptions {
    /// Create new file transfer options with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Hold target file timestamp
    pub fn hold_timestamp(mut self, enable: bool) -> Self {
        self.hold_timestamp = enable;
        self
    }

    /// Enable sync mode (only update newer files)
    pub fn sync_mode(mut self, enable: bool) -> Self {
        self.sync_mode = enable;
        self
    }

    /// Enable compression during transfer
    pub fn compress(mut self, enable: bool) -> Self {
        self.compress = enable;
        self
    }

    /// Enable mode sync
    pub fn mode_sync(mut self, enable: bool) -> Self {
        self.mode_sync = enable;
        self
    }

    /// Send/receive to debug application directory
    pub fn debug_dir(mut self, enable: bool) -> Self {
        self.debug_dir = enable;
        self
    }

    /// Convert options to command flags string
    pub(crate) fn to_flags(&self) -> String {
        let mut flags = Vec::new();

        if self.hold_timestamp {
            flags.push("-a");
        }
        if self.sync_mode {
            flags.push("-sync");
        }
        if self.compress {
            flags.push("-z");
        }
        if self.mode_sync {
            flags.push("-m");
        }
        if self.debug_dir {
            flags.push("-b");
        }

        flags.join(" ")
    }
}

/// File transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTransferDirection {
    /// Send file from local to remote device
    Send,
    /// Receive file from remote device to local
    Recv,
}

/// Validate file path for transfer
pub(crate) fn validate_path(path: &str) -> bool {
    !path.is_empty() && !path.contains('\0')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_options_flags() {
        let opts = FileTransferOptions::new()
            .hold_timestamp(true)
            .compress(true);
        assert_eq!(opts.to_flags(), "-a -z");

        let opts = FileTransferOptions::new().sync_mode(true).mode_sync(true);
        assert_eq!(opts.to_flags(), "-sync -m");
    }

    #[test]
    fn test_validate_path() {
        assert!(validate_path("/data/local/tmp/test.txt"));
        assert!(validate_path("test.txt"));
        assert!(!validate_path(""));
        assert!(!validate_path("test\0file"));
    }
}
