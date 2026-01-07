//! HDC command definitions

/// HDC command codes
///
/// These match the enum in `src/common/define_enum.h`
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HdcCommand {
    // Core commands
    KernelHelp = 0,
    KernelHandshake = 1,
    KernelChannelClose = 2,
    KernelTargetDiscover = 4,
    KernelTargetList = 5,
    KernelTargetAny = 6,
    KernelTargetConnect = 7,
    KernelTargetDisconnect = 8,
    KernelEcho = 9,
    KernelEchoRaw = 10,
    KernelEnableKeepalive = 11,
    KernelWakeupSlavetask = 12,
    CheckServer = 13,
    CheckDevice = 14,
    WaitFor = 15,
    ServerKill = 16,
    ServiceStart = 17,

    // Unity commands (simple one-pass commands)
    UnityExecute = 1001,
    UnityRemount = 1002,
    UnityReboot = 1003,
    UnityRunmode = 1004,
    UnityHilog = 1005,
    UnityRootrun = 1007,
    JdwpList = 1008,
    JdwpTrack = 1009,

    // Shell commands
    ShellInit = 2000,
    ShellData = 2001,

    // Forward commands
    ForwardInit = 2500,
    ForwardCheck = 2501,
    ForwardCheckResult = 2502,
    ForwardActiveSlave = 2503,
    ForwardActiveMaster = 2504,
    ForwardData = 2505,
    ForwardFreeContext = 2506,
    ForwardList = 2507,
    ForwardRemove = 2508,
    ForwardSuccess = 2509,

    // File commands
    FileInit = 3000,
    FileCheck = 3001,
    FileBegin = 3002,
    FileData = 3003,
    FileFinish = 3004,
    AppSideload = 3005,
    FileMode = 3006,
    DirMode = 3007,

    // App commands
    AppInit = 3500,
    AppCheck = 3501,
    AppBegin = 3502,
    AppData = 3503,
    AppFinish = 3504,
    AppUninstall = 3506,

    // Heartbeat
    HeartbeatMsg = 5000,
}

impl HdcCommand {
    /// Convert command to u16 value
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// Convert u16 to command (if valid)
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0 => Some(Self::KernelHelp),
            1 => Some(Self::KernelHandshake),
            2 => Some(Self::KernelChannelClose),
            4 => Some(Self::KernelTargetDiscover),
            5 => Some(Self::KernelTargetList),
            6 => Some(Self::KernelTargetAny),
            7 => Some(Self::KernelTargetConnect),
            8 => Some(Self::KernelTargetDisconnect),
            9 => Some(Self::KernelEcho),
            10 => Some(Self::KernelEchoRaw),
            13 => Some(Self::CheckServer),
            14 => Some(Self::CheckDevice),
            1001 => Some(Self::UnityExecute),
            1002 => Some(Self::UnityRemount),
            1003 => Some(Self::UnityReboot),
            2000 => Some(Self::ShellInit),
            2001 => Some(Self::ShellData),
            3000 => Some(Self::FileInit),
            3003 => Some(Self::FileData),
            3004 => Some(Self::FileFinish),
            _ => None,
        }
    }

    /// Check if this is a response command (has command prefix)
    pub fn is_response(&self) -> bool {
        matches!(
            self,
            Self::ShellData
                | Self::FileData
                | Self::FileFinish
                | Self::ForwardData
                | Self::KernelEcho
        )
    }
}
