/// Target boot mode accepted by `target boot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetBootMode {
    /// Boot to bootloader mode.
    Bootloader,
    /// Boot to recovery mode.
    Recovery,
    /// Pass a custom upstream boot mode argument through unchanged.
    Other(String),
}

impl TargetBootMode {
    /// Render this mode as an upstream `target boot` argument.
    pub fn as_arg(&self) -> &str {
        match self {
            Self::Bootloader => "-bootloader",
            Self::Recovery => "-recovery",
            Self::Other(value) => value.as_str(),
        }
    }
}

impl From<&str> for TargetBootMode {
    fn from(value: &str) -> Self {
        match value {
            "-bootloader" | "bootloader" => Self::Bootloader,
            "-recovery" | "recovery" => Self::Recovery,
            other => Self::Other(other.to_string()),
        }
    }
}

/// Target transport mode accepted by `tmode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetMode {
    /// Switch target to USB mode.
    Usb,
    /// Switch target to TCP port mode. `None` renders `tmode port`.
    Port(Option<u16>),
    /// Close TCP port mode.
    PortClose,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_boot_modes_render_upstream_args() {
        assert_eq!(TargetBootMode::Bootloader.as_arg(), "-bootloader");
        assert_eq!(TargetBootMode::Recovery.as_arg(), "-recovery");
        assert_eq!(
            TargetBootMode::Other("flashd".to_string()).as_arg(),
            "flashd"
        );
    }
}
