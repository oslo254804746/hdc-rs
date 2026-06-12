//! Application management functionality

/// Application install options
#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    /// Replace existing application
    pub replace: bool,
    /// Install shared bundle for multi-apps
    pub shared: bool,
    /// Execute install relative to working directory (-cwd)
    pub cwd: Option<String>,
    /// Wait time in seconds (-w)
    pub wait_time: Option<u64>,
    /// User ID (-u)
    pub user_id: Option<String>,
    /// Bundle path (-p)
    pub bundle_path: Option<String>,
    /// List install options/help (-h)
    pub list_options: bool,
    /// Grant permissions after install (-g)
    pub grant_permissions: bool,
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

    /// Set install working directory
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set install wait time
    pub fn wait_time(mut self, wait_time: u64) -> Self {
        self.wait_time = Some(wait_time);
        self
    }

    /// Set user ID
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set bundle path
    pub fn bundle_path(mut self, bundle_path: impl Into<String>) -> Self {
        self.bundle_path = Some(bundle_path.into());
        self
    }

    /// Set list-options/help flag
    pub fn list_options(mut self, list_options: bool) -> Self {
        self.list_options = list_options;
        self
    }

    /// Set grant-permissions flag
    pub fn grant_permissions(mut self, grant_permissions: bool) -> Self {
        self.grant_permissions = grant_permissions;
        self
    }

    /// Convert to command line flags
    pub fn to_flags(&self) -> String {
        let mut flags = Vec::new();
        if self.replace {
            flags.push("-r".to_string());
        }
        if self.shared {
            flags.push("-s".to_string());
        }
        if let Some(cwd) = &self.cwd {
            flags.push("-cwd".to_string());
            flags.push(cwd.clone());
        }
        if let Some(wait_time) = self.wait_time {
            flags.push("-w".to_string());
            flags.push(wait_time.to_string());
        }
        if let Some(user_id) = &self.user_id {
            flags.push("-u".to_string());
            flags.push(user_id.clone());
        }
        if let Some(bundle_path) = &self.bundle_path {
            flags.push("-p".to_string());
            flags.push(bundle_path.clone());
        }
        if self.list_options {
            flags.push("-h".to_string());
        }
        if self.grant_permissions {
            flags.push("-g".to_string());
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
    /// Bundle name (-n)
    pub bundle_name: Option<String>,
    /// Module name (-m)
    pub module_name: Option<String>,
    /// Version code (-v)
    pub version_code: Option<String>,
    /// User ID (-u)
    pub user_id: Option<String>,
    /// List uninstall options/help (-h)
    pub list_options: bool,
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

    /// Set bundle name
    pub fn bundle_name(mut self, bundle_name: impl Into<String>) -> Self {
        self.bundle_name = Some(bundle_name.into());
        self
    }

    /// Set module name
    pub fn module_name(mut self, module_name: impl Into<String>) -> Self {
        self.module_name = Some(module_name.into());
        self
    }

    /// Set version code
    pub fn version_code(mut self, version_code: impl Into<String>) -> Self {
        self.version_code = Some(version_code.into());
        self
    }

    /// Set user ID
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set list-options/help flag
    pub fn list_options(mut self, list_options: bool) -> Self {
        self.list_options = list_options;
        self
    }

    /// Convert to command line flags
    pub fn to_flags(&self) -> String {
        let mut flags = Vec::new();
        if self.keep_data {
            flags.push("-k".to_string());
        }
        if self.shared {
            flags.push("-s".to_string());
        }
        if let Some(bundle_name) = &self.bundle_name {
            flags.push("-n".to_string());
            flags.push(bundle_name.clone());
        }
        if let Some(module_name) = &self.module_name {
            flags.push("-m".to_string());
            flags.push(module_name.clone());
        }
        if let Some(version_code) = &self.version_code {
            flags.push("-v".to_string());
            flags.push(version_code.clone());
        }
        if let Some(user_id) = &self.user_id {
            flags.push("-u".to_string());
            flags.push(user_id.clone());
        }
        if self.list_options {
            flags.push("-h".to_string());
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
    fn test_install_options_extended_flags() {
        let opts = InstallOptions::new()
            .cwd("/tmp")
            .wait_time(30)
            .user_id("100")
            .bundle_path("/data/app")
            .list_options(true)
            .grant_permissions(true);
        assert_eq!(opts.to_flags(), "-cwd /tmp -w 30 -u 100 -p /data/app -h -g");
    }

    #[test]
    fn test_uninstall_options() {
        let opts = UninstallOptions::new().keep_data(true);
        assert_eq!(opts.to_flags(), "-k");

        let opts = UninstallOptions::new().keep_data(true).shared(true);
        assert_eq!(opts.to_flags(), "-k -s");
    }

    #[test]
    fn test_uninstall_options_extended_flags() {
        let opts = UninstallOptions::new()
            .bundle_name("com.example.app")
            .module_name("entry")
            .version_code("42")
            .user_id("100")
            .list_options(true);
        assert_eq!(
            opts.to_flags(),
            "-n com.example.app -m entry -v 42 -u 100 -h"
        );
    }
}
