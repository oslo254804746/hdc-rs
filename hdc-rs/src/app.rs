//! Application management functionality

use crate::error::{HdcError, Result};

/// Validate a working-directory value, which remains a separate HDC argument.
pub(crate) fn validate_cwd_value(option: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value
            .bytes()
            .any(|byte| matches!(byte, b'"' | b'\r' | b'\n' | b'\0'))
    {
        return Err(HdcError::Protocol(format!(
            "invalid {option} value: it must be non-empty and cannot contain double quotes, CR, LF, or NUL"
        )));
    }
    Ok(())
}

/// Validate a daemon/package-manager option value before quoting it.
pub(crate) fn validate_option_value(option: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.bytes().any(|byte| {
            byte.is_ascii_whitespace()
                || byte.is_ascii_control()
                || matches!(
                    byte,
                    b'"' | b'|' | b';' | b'&' | b'$' | b'<' | b'>' | b'`' | b'\\' | b'!'
                )
        })
    {
        return Err(HdcError::Protocol(format!(
            "invalid {option} value: it must be non-empty and cannot contain whitespace, control, quote, or shell-injection characters"
        )));
    }
    Ok(())
}

pub(crate) fn validate_numeric_option_value(option: &str, value: &str) -> Result<()> {
    validate_option_value(option, value)?;
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(HdcError::Protocol(format!(
            "invalid {option} value: only ASCII digits are allowed"
        )));
    }
    Ok(())
}

pub(crate) fn validate_module_name(value: &str) -> Result<()> {
    validate_option_value("-m", value)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(HdcError::Protocol(
            "invalid -m value: only ASCII letters, digits, '.', '_', and '-' are allowed"
                .to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_package_name(value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(HdcError::Protocol(
            "invalid package value: only ASCII letters, digits, '.', '_', and '-' are allowed"
                .to_string(),
        ));
    }
    Ok(())
}

/// Validate a source/path argument and reject values that cannot be quoted.
pub(crate) fn validate_path_argument(label: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value
            .bytes()
            .any(|byte| matches!(byte, b'"' | b'\r' | b'\n' | b'\0'))
    {
        return Err(HdcError::Protocol(format!(
            "invalid {label}: it must be non-empty and cannot contain double quotes, CR, LF, or NUL"
        )));
    }
    Ok(())
}

/// Render one host command argument, quoting only when it contains whitespace.
pub(crate) fn render_hdc_argument(value: &str) -> String {
    if value.bytes().any(|byte| byte.is_ascii_whitespace()) {
        format!("\"{value}\"")
    } else {
        value.to_string()
    }
}

/// Keep `-cwd` and its value as separate HDC arguments while preserving paths
/// containing whitespace. The value itself is quoted only when needed.
pub(crate) fn render_cwd_value(value: &str) -> String {
    render_hdc_argument(value)
}

fn render_option_pair(option: &str, value: &str) -> String {
    format!("\"{option} {value}\"")
}

/// Application install options
#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    /// Replace existing application
    pub replace: bool,
    /// Install shared bundle for multi-apps
    pub shared: bool,
    /// Execute install relative to working directory (-cwd). The value must
    /// be non-empty and must not contain double quotes, CR, LF, or NUL.
    pub cwd: Option<String>,
    /// Wait time in seconds (-w)
    pub wait_time: Option<u64>,
    /// User ID (-u), encoded as a quoted option/value argument.
    pub user_id: Option<String>,
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

    /// Set an ASCII-numeric user ID.
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
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

    /// Validate values before sending an install command.
    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(cwd) = &self.cwd {
            validate_cwd_value("-cwd", cwd)?;
        }
        if let Some(user_id) = &self.user_id {
            validate_numeric_option_value("-u", user_id)?;
        }
        Ok(())
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
            flags.push(render_cwd_value(cwd));
        }
        if let Some(wait_time) = self.wait_time {
            flags.push(format!("\"-w {wait_time}\""));
        }
        if let Some(user_id) = &self.user_id {
            flags.push(render_option_pair("-u", user_id));
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
    /// Module name (-m), encoded as a quoted option/value argument.
    pub module_name: Option<String>,
    /// Version code (-v), encoded as a quoted option/value argument.
    pub version_code: Option<String>,
    /// User ID (-u), encoded as a quoted option/value argument.
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

    /// Validate values before sending an uninstall command.
    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(module_name) = &self.module_name {
            validate_module_name(module_name)?;
        }
        if let Some(version_code) = &self.version_code {
            validate_numeric_option_value("-v", version_code)?;
        }
        if let Some(user_id) = &self.user_id {
            validate_numeric_option_value("-u", user_id)?;
        }
        Ok(())
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
        if let Some(module_name) = &self.module_name {
            flags.push(render_option_pair("-m", module_name));
        }
        if let Some(version_code) = &self.version_code {
            flags.push(render_option_pair("-v", version_code));
        }
        if let Some(user_id) = &self.user_id {
            flags.push(render_option_pair("-u", user_id));
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
            .list_options(true)
            .grant_permissions(true);
        assert_eq!(opts.to_flags(), "-cwd /tmp \"-w 30\" \"-u 100\" -h -g");

        let opts = InstallOptions::new().cwd("/tmp/work dir");
        assert_eq!(opts.to_flags(), "-cwd \"/tmp/work dir\"");

        let opts = InstallOptions::new().wait_time(180);
        assert_eq!(opts.to_flags(), "\"-w 180\"");
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
            .module_name("entry")
            .version_code("42")
            .user_id("100")
            .list_options(true);
        assert_eq!(opts.to_flags(), "\"-m entry\" \"-v 42\" \"-u 100\" -h");

        let opts = UninstallOptions::new()
            .module_name("entry_module")
            .version_code("42")
            .user_id("100");
        assert_eq!(opts.to_flags(), "\"-m entry_module\" \"-v 42\" \"-u 100\"");
    }

    #[test]
    fn reject_unsafe_option_values() {
        for value in ["bad\"value", "bad\nvalue", "bad\rvalue", "bad\0value"] {
            assert!(InstallOptions::new().user_id(value).validate().is_err());
            assert!(UninstallOptions::new()
                .module_name(value)
                .validate()
                .is_err());
        }

        assert!(InstallOptions::new().cwd("safe value").validate().is_ok());
        assert!(InstallOptions::new().user_id("100").validate().is_ok());
        assert!(InstallOptions::new().user_id("10 0").validate().is_err());
        assert!(UninstallOptions::new()
            .module_name("entry")
            .validate()
            .is_ok());
        assert!(UninstallOptions::new()
            .module_name("entry module")
            .validate()
            .is_err());
        assert!(UninstallOptions::new()
            .version_code("42")
            .user_id("100")
            .validate()
            .is_ok());
    }
}
