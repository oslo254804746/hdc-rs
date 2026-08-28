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
    KernelTargetReconnect = 18,
    SslHandshake = 20,

    // Unity commands (simple one-pass commands)
    UnityExecute = 1001,
    UnityRemount = 1002,
    UnityReboot = 1003,
    UnityRunmode = 1004,
    UnityHilog = 1005,
    UnityRootrun = 1007,
    JdwpList = 1008,
    JdwpTrack = 1009,
    UnityBugreportInit = 1011,
    UnityBugreportData = 1012,
    UnityExecuteEx = 1200,

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

    // Flashd commands
    FlashdUpdateInit = 4000,
    FlashdFlashInit = 4001,
    FlashdCheck = 4002,
    FlashdBegin = 4003,
    FlashdData = 4004,
    FlashdFinish = 4005,
    FlashdErase = 4006,
    FlashdFormat = 4007,
    FlashdProgress = 4008,

    // Heartbeat
    HeartbeatMsg = 5000,

    // Spawn-sub commands
    SpawnSub = 6000,
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
            11 => Some(Self::KernelEnableKeepalive),
            12 => Some(Self::KernelWakeupSlavetask),
            13 => Some(Self::CheckServer),
            14 => Some(Self::CheckDevice),
            15 => Some(Self::WaitFor),
            16 => Some(Self::ServerKill),
            17 => Some(Self::ServiceStart),
            18 => Some(Self::KernelTargetReconnect),
            20 => Some(Self::SslHandshake),
            1001 => Some(Self::UnityExecute),
            1002 => Some(Self::UnityRemount),
            1003 => Some(Self::UnityReboot),
            1004 => Some(Self::UnityRunmode),
            1005 => Some(Self::UnityHilog),
            1007 => Some(Self::UnityRootrun),
            1008 => Some(Self::JdwpList),
            1009 => Some(Self::JdwpTrack),
            1011 => Some(Self::UnityBugreportInit),
            1012 => Some(Self::UnityBugreportData),
            1200 => Some(Self::UnityExecuteEx),
            2000 => Some(Self::ShellInit),
            2001 => Some(Self::ShellData),
            2500 => Some(Self::ForwardInit),
            2501 => Some(Self::ForwardCheck),
            2502 => Some(Self::ForwardCheckResult),
            2503 => Some(Self::ForwardActiveSlave),
            2504 => Some(Self::ForwardActiveMaster),
            2505 => Some(Self::ForwardData),
            2506 => Some(Self::ForwardFreeContext),
            2507 => Some(Self::ForwardList),
            2508 => Some(Self::ForwardRemove),
            2509 => Some(Self::ForwardSuccess),
            3000 => Some(Self::FileInit),
            3001 => Some(Self::FileCheck),
            3002 => Some(Self::FileBegin),
            3003 => Some(Self::FileData),
            3004 => Some(Self::FileFinish),
            3005 => Some(Self::AppSideload),
            3006 => Some(Self::FileMode),
            3007 => Some(Self::DirMode),
            3500 => Some(Self::AppInit),
            3501 => Some(Self::AppCheck),
            3502 => Some(Self::AppBegin),
            3503 => Some(Self::AppData),
            3504 => Some(Self::AppFinish),
            3506 => Some(Self::AppUninstall),
            4000 => Some(Self::FlashdUpdateInit),
            4001 => Some(Self::FlashdFlashInit),
            4002 => Some(Self::FlashdCheck),
            4003 => Some(Self::FlashdBegin),
            4004 => Some(Self::FlashdData),
            4005 => Some(Self::FlashdFinish),
            4006 => Some(Self::FlashdErase),
            4007 => Some(Self::FlashdFormat),
            4008 => Some(Self::FlashdProgress),
            5000 => Some(Self::HeartbeatMsg),
            6000 => Some(Self::SpawnSub),
            _ => None,
        }
    }

    /// Check if this is a response command (has command prefix)
    pub fn is_response(&self) -> bool {
        matches!(
            self,
            Self::CheckServer
                | Self::ShellData
                | Self::FileData
                | Self::FileFinish
                | Self::ForwardData
                | Self::KernelEcho
        )
    }
}

#[cfg(test)]
mod tests {
    use super::HdcCommand;

    const ALL_COMMANDS: &[HdcCommand] = &[
        HdcCommand::KernelHelp,
        HdcCommand::KernelHandshake,
        HdcCommand::KernelChannelClose,
        HdcCommand::KernelTargetDiscover,
        HdcCommand::KernelTargetList,
        HdcCommand::KernelTargetAny,
        HdcCommand::KernelTargetConnect,
        HdcCommand::KernelTargetDisconnect,
        HdcCommand::KernelEcho,
        HdcCommand::KernelEchoRaw,
        HdcCommand::KernelEnableKeepalive,
        HdcCommand::KernelWakeupSlavetask,
        HdcCommand::CheckServer,
        HdcCommand::CheckDevice,
        HdcCommand::WaitFor,
        HdcCommand::ServerKill,
        HdcCommand::ServiceStart,
        HdcCommand::KernelTargetReconnect,
        HdcCommand::SslHandshake,
        HdcCommand::UnityExecute,
        HdcCommand::UnityRemount,
        HdcCommand::UnityReboot,
        HdcCommand::UnityRunmode,
        HdcCommand::UnityHilog,
        HdcCommand::UnityRootrun,
        HdcCommand::JdwpList,
        HdcCommand::JdwpTrack,
        HdcCommand::UnityBugreportInit,
        HdcCommand::UnityBugreportData,
        HdcCommand::UnityExecuteEx,
        HdcCommand::ShellInit,
        HdcCommand::ShellData,
        HdcCommand::ForwardInit,
        HdcCommand::ForwardCheck,
        HdcCommand::ForwardCheckResult,
        HdcCommand::ForwardActiveSlave,
        HdcCommand::ForwardActiveMaster,
        HdcCommand::ForwardData,
        HdcCommand::ForwardFreeContext,
        HdcCommand::ForwardList,
        HdcCommand::ForwardRemove,
        HdcCommand::ForwardSuccess,
        HdcCommand::FileInit,
        HdcCommand::FileCheck,
        HdcCommand::FileBegin,
        HdcCommand::FileData,
        HdcCommand::FileFinish,
        HdcCommand::AppSideload,
        HdcCommand::FileMode,
        HdcCommand::DirMode,
        HdcCommand::AppInit,
        HdcCommand::AppCheck,
        HdcCommand::AppBegin,
        HdcCommand::AppData,
        HdcCommand::AppFinish,
        HdcCommand::AppUninstall,
        HdcCommand::FlashdUpdateInit,
        HdcCommand::FlashdFlashInit,
        HdcCommand::FlashdCheck,
        HdcCommand::FlashdBegin,
        HdcCommand::FlashdData,
        HdcCommand::FlashdFinish,
        HdcCommand::FlashdErase,
        HdcCommand::FlashdFormat,
        HdcCommand::FlashdProgress,
        HdcCommand::HeartbeatMsg,
        HdcCommand::SpawnSub,
    ];

    #[test]
    fn upstream_numeric_values_are_stable() {
        let cases = [
            (HdcCommand::KernelTargetReconnect, 18),
            (HdcCommand::SslHandshake, 20),
            (HdcCommand::UnityBugreportInit, 1011),
            (HdcCommand::UnityBugreportData, 1012),
            (HdcCommand::UnityExecuteEx, 1200),
            (HdcCommand::FlashdUpdateInit, 4000),
            (HdcCommand::FlashdFlashInit, 4001),
            (HdcCommand::FlashdCheck, 4002),
            (HdcCommand::FlashdBegin, 4003),
            (HdcCommand::FlashdData, 4004),
            (HdcCommand::FlashdFinish, 4005),
            (HdcCommand::FlashdErase, 4006),
            (HdcCommand::FlashdFormat, 4007),
            (HdcCommand::FlashdProgress, 4008),
            (HdcCommand::HeartbeatMsg, 5000),
            (HdcCommand::SpawnSub, 6000),
        ];

        for (command, value) in cases {
            assert_eq!(command.as_u16(), value);
            assert_eq!(HdcCommand::from_u16(value), Some(command));
        }
    }

    #[test]
    fn every_command_round_trips_through_wire_value() {
        for &command in ALL_COMMANDS {
            assert_eq!(HdcCommand::from_u16(command.as_u16()), Some(command));
        }
    }

    #[test]
    fn unknown_wire_values_return_none() {
        assert_eq!(HdcCommand::from_u16(3), None);
        assert_eq!(HdcCommand::from_u16(19), None);
        assert_eq!(HdcCommand::from_u16(u16::MAX), None);
    }

    #[test]
    fn only_check_server_is_a_core_response_prefix() {
        assert!(HdcCommand::CheckServer.is_response());
        assert!(!HdcCommand::CheckDevice.is_response());
    }
}
