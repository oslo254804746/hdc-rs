//! Application management functionality

/// Application install options
#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    /// Replace existing application
    pub replace: bool,
    /// Install shared bundle for multi-apps
    pub shared: bool,
}

impl InstallOptions {
    /// Create default install options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set replace option
    pub fn replace(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }

    /// Set shared option
    pub fn shared(mut self, shared: bool) -> Self {
        self.shared = shared;
        self
    }

    /// Convert to command line flags
    pub fn to_flags(&self) -> String {
        let mut flags = Vec::new();
        if self.replace {
            flags.push("-r");
        }
        if self.shared {
            flags.push("-s");
        }
        flags.join(" ")
    }
}

/// Application uninstall options
#[derive(Debug, Clone, Default)]
pub struct UninstallOptions {
    /// Keep the data and cache directories
    pub keep_data: bool,
    /// Remove shared bundle
    pub shared: bool,
}

impl UninstallOptions {
    /// Create default uninstall options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set keep_data option
    pub fn keep_data(mut self, keep: bool) -> Self {
        self.keep_data = keep;
        self
    }

    /// Set shared option
    pub fn shared(mut self, shared: bool) -> Self {
        self.shared = shared;
        self
    }

    /// Convert to command line flags
    pub fn to_flags(&self) -> String {
        let mut flags = Vec::new();
        if self.keep_data {
            flags.push("-k");
        }
        if self.shared {
            flags.push("-s");
        }
        flags.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_options() {
        let opts = InstallOptions::new().replace(true);
        assert_eq!(opts.to_flags(), "-r");

        let opts = InstallOptions::new().replace(true).shared(true);
        assert_eq!(opts.to_flags(), "-r -s");
    }

    #[test]
    fn test_uninstall_options() {
        let opts = UninstallOptions::new().keep_data(true);
        assert_eq!(opts.to_flags(), "-k");

        let opts = UninstallOptions::new().keep_data(true).shared(true);
        assert_eq!(opts.to_flags(), "-k -s");
    }
}
