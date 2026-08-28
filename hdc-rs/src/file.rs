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
    /// Execute file transfer relative to working directory (-cwd). The value
    /// must not contain double quotes, CR, LF, or NUL.
    cwd: Option<String>,
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

    /// Set transfer working directory
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Validate values before sending a file-transfer command.
    pub(crate) fn validate(&self) -> crate::error::Result<()> {
        if let Some(cwd) = &self.cwd {
            crate::app::validate_cwd_value("-cwd", cwd)?;
        }
        Ok(())
    }

    /// Convert options to command flags string
    pub(crate) fn to_flags(&self) -> String {
        let mut flags: Vec<String> = Vec::new();

        if self.hold_timestamp {
            flags.push("-a".to_string());
        }
        if self.sync_mode {
            flags.push("-sync".to_string());
        }
        if self.compress {
            flags.push("-z".to_string());
        }
        if self.mode_sync {
            flags.push("-m".to_string());
        }
        if self.debug_dir {
            flags.push("-b".to_string());
        }
        if let Some(cwd) = &self.cwd {
            flags.push("-cwd".to_string());
            flags.push(crate::app::render_cwd_value(cwd));
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
    fn test_file_options_render_debug_dir_and_cwd() {
        let opts = FileTransferOptions::new()
            .debug_dir(true)
            .cwd("/data/local/tmp");
        assert_eq!(opts.to_flags(), "-b -cwd /data/local/tmp");

        let opts = FileTransferOptions::new().cwd("/data/local/tmp with space");
        assert_eq!(opts.to_flags(), "-cwd \"/data/local/tmp with space\"");
    }

    #[test]
    fn test_reject_unsafe_cwd() {
        for value in ["bad\"path", "bad\npath", "bad\rpath", "bad\0path"] {
            assert!(FileTransferOptions::new().cwd(value).validate().is_err());
        }
    }

    #[test]
    fn test_validate_path() {
        assert!(validate_path("/data/local/tmp/test.txt"));
        assert!(validate_path("test.txt"));
        assert!(!validate_path(""));
        assert!(!validate_path("test\0file"));
    }
}
