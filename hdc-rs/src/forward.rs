//! Port forwarding functionality

/// Forward node type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForwardNode {
    /// TCP port: tcp:port
    Tcp(u16),
    /// Local filesystem Unix domain socket: localfilesystem:name
    LocalFilesystem(String),
    /// Local reserved Unix domain socket: localreserved:name
    LocalReserved(String),
    /// Local abstract Unix domain socket: localabstract:name
    LocalAbstract(String),
    /// Device: dev:name
    Dev(String),
    /// JDWP process (remote only): jdwp:pid
    Jdwp(u32),
    /// Ark debugger (remote only): ark:pid@tid@Debugger
    Ark {
        pid: u32,
        tid: u32,
        debugger: String,
    },
}

impl ForwardNode {
    /// Parse a forward node from string format
    ///
    /// Format examples:
    /// - `tcp:8080`
    /// - `localfilesystem:/tmp/socket`
    /// - `localreserved:name`
    /// - `localabstract:name`
    /// - `dev:device_name`
    /// - `jdwp:1234`
    /// - `ark:1234@5678@Debugger`
    pub fn parse(s: &str) -> crate::error::Result<Self> {
        if let Some(port_str) = s.strip_prefix("tcp:") {
            let port = port_str.parse::<u16>().map_err(|_| {
                crate::error::HdcError::Protocol(format!("Invalid TCP port: {}", port_str))
            })?;
            Ok(Self::Tcp(port))
        } else if let Some(name) = s.strip_prefix("localfilesystem:") {
            Ok(Self::LocalFilesystem(name.to_string()))
        } else if let Some(name) = s.strip_prefix("localreserved:") {
            Ok(Self::LocalReserved(name.to_string()))
        } else if let Some(name) = s.strip_prefix("localabstract:") {
            Ok(Self::LocalAbstract(name.to_string()))
        } else if let Some(name) = s.strip_prefix("dev:") {
            Ok(Self::Dev(name.to_string()))
        } else if let Some(pid_str) = s.strip_prefix("jdwp:") {
            let pid = pid_str.parse::<u32>().map_err(|_| {
                crate::error::HdcError::Protocol(format!("Invalid JDWP pid: {}", pid_str))
            })?;
            Ok(Self::Jdwp(pid))
        } else if let Some(ark_str) = s.strip_prefix("ark:") {
            let parts: Vec<&str> = ark_str.split('@').collect();
            if parts.len() != 3 {
                return Err(crate::error::HdcError::Protocol(format!(
                    "Invalid ark format: expected pid@tid@debugger, got {}",
                    ark_str
                )));
            }
            let pid = parts[0].parse::<u32>().map_err(|_| {
                crate::error::HdcError::Protocol(format!("Invalid pid in ark: {}", parts[0]))
            })?;
            let tid = parts[1].parse::<u32>().map_err(|_| {
                crate::error::HdcError::Protocol(format!("Invalid tid in ark: {}", parts[1]))
            })?;
            Ok(Self::Ark {
                pid,
                tid,
                debugger: parts[2].to_string(),
            })
        } else {
            Err(crate::error::HdcError::Protocol(format!(
                "Invalid forward node format: {}",
                s
            )))
        }
    }

    /// Convert to protocol string representation
    pub fn as_protocol_string(&self) -> String {
        match self {
            Self::Tcp(port) => format!("tcp:{}", port),
            Self::LocalFilesystem(name) => format!("localfilesystem:{}", name),
            Self::LocalReserved(name) => format!("localreserved:{}", name),
            Self::LocalAbstract(name) => format!("localabstract:{}", name),
            Self::Dev(name) => format!("dev:{}", name),
            Self::Jdwp(pid) => format!("jdwp:{}", pid),
            Self::Ark { pid, tid, debugger } => format!("ark:{}@{}@{}", pid, tid, debugger),
        }
    }
}

/// Forward task information
#[derive(Debug, Clone)]
pub struct ForwardTask {
    pub local_node: ForwardNode,
    pub remote_node: ForwardNode,
    pub is_forward: bool, // true for fport, false for rport
}

impl ForwardTask {
    /// Create a forward (fport) task
    pub fn forward(local: ForwardNode, remote: ForwardNode) -> Self {
        Self {
            local_node: local,
            remote_node: remote,
            is_forward: true,
        }
    }

    /// Create a reverse (rport) task
    pub fn reverse(remote: ForwardNode, local: ForwardNode) -> Self {
        Self {
            local_node: local,
            remote_node: remote,
            is_forward: false,
        }
    }

    /// Convert to command string format
    pub fn to_command_string(&self) -> String {
        if self.is_forward {
            format!(
                "fport {} {}",
                self.local_node.as_protocol_string(),
                self.remote_node.as_protocol_string()
            )
        } else {
            format!(
                "rport {} {}",
                self.remote_node.as_protocol_string(),
                self.local_node.as_protocol_string()
            )
        }
    }

    /// Get task string (for removal)
    pub fn task_string(&self) -> String {
        format!(
            "{} {}",
            self.local_node.as_protocol_string(),
            self.remote_node.as_protocol_string()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tcp() {
        let node = ForwardNode::parse("tcp:8080").unwrap();
        assert_eq!(node, ForwardNode::Tcp(8080));
        assert_eq!(node.as_protocol_string(), "tcp:8080");
    }

    #[test]
    fn test_parse_jdwp() {
        let node = ForwardNode::parse("jdwp:1234").unwrap();
        assert_eq!(node, ForwardNode::Jdwp(1234));
        assert_eq!(node.as_protocol_string(), "jdwp:1234");
    }

    #[test]
    fn test_parse_ark() {
        let node = ForwardNode::parse("ark:100@200@Debugger").unwrap();
        let expected = ForwardNode::Ark {
            pid: 100,
            tid: 200,
            debugger: "Debugger".to_string(),
        };
        assert_eq!(node, expected);
        assert_eq!(node.as_protocol_string(), "ark:100@200@Debugger");
    }

    #[test]
    fn test_forward_task() {
        let task = ForwardTask::forward(ForwardNode::Tcp(8080), ForwardNode::Tcp(8081));
        assert_eq!(task.to_command_string(), "fport tcp:8080 tcp:8081");
        assert_eq!(task.task_string(), "tcp:8080 tcp:8081");
    }
}
