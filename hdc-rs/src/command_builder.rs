pub fn target_connect(key: &str, remove: bool) -> String {
    if remove {
        format!("tconn {} -remove", key)
    } else {
        format!("tconn {}", key)
    }
}

pub fn reconnect_target(connect_key: Option<&str>) -> String {
    match connect_key {
        Some(key) if !key.is_empty() => format!("reconnect {}", key),
        _ => "reconnect".to_string(),
    }
}

pub fn target_mount() -> String {
    "target mount".to_string()
}

pub fn target_boot(mode: Option<&str>) -> String {
    match mode {
        Some(mode) if !mode.is_empty() => format!("target boot {}", mode),
        _ => "target boot".to_string(),
    }
}

pub fn smode(enable_root: bool) -> String {
    if enable_root {
        "smode".to_string()
    } else {
        "smode -r".to_string()
    }
}

pub fn tmode_usb() -> String {
    "tmode usb".to_string()
}

pub fn tmode_port(port: Option<u16>) -> String {
    match port {
        Some(port) => format!("tmode port {}", port),
        None => "tmode port".to_string(),
    }
}

pub fn tmode_port_close() -> String {
    "tmode port close".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_target_connection_commands() {
        assert_eq!(
            target_connect("192.168.0.2:10178", false),
            "tconn 192.168.0.2:10178"
        );
        assert_eq!(
            target_connect("192.168.0.2:10178", true),
            "tconn 192.168.0.2:10178 -remove"
        );
        assert_eq!(reconnect_target(Some("SERIAL")), "reconnect SERIAL");
        assert_eq!(reconnect_target(None), "reconnect");
    }

    #[test]
    fn renders_device_control_commands() {
        assert_eq!(target_mount(), "target mount");
        assert_eq!(target_boot(None), "target boot");
        assert_eq!(target_boot(Some("recovery")), "target boot recovery");
        assert_eq!(smode(true), "smode");
        assert_eq!(smode(false), "smode -r");
        assert_eq!(tmode_usb(), "tmode usb");
        assert_eq!(tmode_port(Some(10178)), "tmode port 10178");
        assert_eq!(tmode_port(None), "tmode port");
        assert_eq!(tmode_port_close(), "tmode port close");
    }
}
