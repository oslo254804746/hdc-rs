# HDC Client API Matrix

This matrix tracks the implemented client-facing upstream HDC commands against
the Rust async API, Rust blocking API, Python API, and test coverage in this
repository. “Implemented” indicates that the API is exposed; it does not imply
real-device validation. Device-dependent integration tests are ignored by
default and require an HDC server plus an authorized HarmonyOS/OpenHarmony
device, so CI results do not claim hardware validation.

| Upstream command | Rust async API | Rust blocking API | Python API | Test coverage | Requires device | Priority |
| --- | --- | --- | --- | --- | --- | --- |
| `version` | `HdcClient::version` | `blocking::HdcClient::version` | `HdcClient.version` | Doc compile | No | High |
| `help [verbose]` | `HdcClient::help` | `blocking::HdcClient::help` | `HdcClient.help` | Doc compile | No | High |
| `discover` | `HdcClient::discover` | `blocking::HdcClient::discover` | `HdcClient.discover` | Compile | No | High |
| `list targets` | `HdcClient::list_targets` | `blocking::HdcClient::list_targets` | `HdcClient.list_targets` | Unit ignored integration | No | High |
| `list targets -v` | `HdcClient::list_targets_verbose` | `blocking::HdcClient::list_targets_verbose` | `HdcClient.list_targets_verbose` | Doc compile | No | Medium |
| `checkserver` | `HdcClient::check_server` | `blocking::HdcClient::check_server` | `HdcClient.check_server` | Compile | No | High |
| `checkdevice [key]` | `HdcClient::check_device` | `blocking::HdcClient::check_device` | `HdcClient.check_device` | Doc compile | Usually | High |
| `wait` | `HdcClient::wait_for_device` | `blocking::HdcClient::wait_for_device` | `HdcClient.wait_for_device` | Doc compile | No | High |
| `tconn key` | `HdcClient::target_connect` | `blocking::HdcClient::target_connect` | `HdcClient.target_connect` | Command builder unit | No | High |
| `tconn key -remove` | `HdcClient::target_disconnect` | `blocking::HdcClient::target_disconnect` | `HdcClient.target_disconnect` | Command builder unit | No | High |
| `any` | `HdcClient::connect_any` | `blocking::HdcClient::connect_any` | `HdcClient.connect_any` | Doc compile | No | High |
| `reconnect [key]` | `HdcClient::reconnect_target` | `blocking::HdcClient::reconnect_target` | `HdcClient.reconnect_target` | Command builder unit | Usually | High |
| `target mount` | `HdcClient::target_mount` | `blocking::HdcClient::target_mount` | `HdcClient.target_mount` | Command builder unit | Yes | High |
| `target boot [mode]` | `HdcClient::target_boot` | `blocking::HdcClient::target_boot` | `HdcClient.target_boot` | Command builder and device unit | Yes | High |
| `smode [-r]` | `HdcClient::smode` | `blocking::HdcClient::smode` | `HdcClient.smode` | Command builder unit | Yes | High |
| `tmode usb` | `HdcClient::tmode(TargetMode::Usb)` | `blocking::HdcClient::tmode` | `HdcClient.tmode_usb` | Command builder unit | Yes | High |
| `tmode port [port]` | `HdcClient::tmode(TargetMode::Port)` | `blocking::HdcClient::tmode` | `HdcClient.tmode_port` | Command builder unit | Yes | High |
| `tmode port close` | `HdcClient::tmode(TargetMode::PortClose)` | `blocking::HdcClient::tmode` | `HdcClient.tmode_port_close` | Command builder unit | Yes | High |
| `shell <command>` | `HdcClient::shell` | `blocking::HdcClient::shell` | `HdcClient.shell` | Ignored integration | Yes | High |
| `hilog [args]` | `HdcClient::hilog`, `hilog_stream` | `blocking::HdcClient::hilog`, `hilog_stream` | `HdcClient.hilog`, `hilog_stream` | Doc compile | Yes | High |
| `jpid` | `HdcClient::jpid` | `blocking::HdcClient::jpid` | `HdcClient.jpid` | Parser unit, ignored integration | Yes | High |
| `track-jpid [-a|-p]` | `HdcClient::track_jpid` | `blocking::HdcClient::track_jpid` | `HdcClient.track_jpid` | Compile | Yes | High |
| `file send` | `HdcClient::file_send` | `blocking::HdcClient::file_send` | `HdcClient.file_send` | Options unit, doc compile | Yes | High |
| `file recv` | `HdcClient::file_recv` | `blocking::HdcClient::file_recv` | `HdcClient.file_recv` | Options unit, doc compile | Yes | High |
| `fport local remote` | `HdcClient::fport` | `blocking::HdcClient::fport` | `HdcClient.fport` | Forward unit, ignored integration | Usually | High |
| `fport ls` | `HdcClient::fport_list` | `blocking::HdcClient::fport_list` | `HdcClient.fport_list` | Ignored integration | No | Medium |
| `fport rm task` | `HdcClient::fport_remove` | `blocking::HdcClient::fport_remove` | `HdcClient.fport_remove` | Ignored integration | No | Medium |
| `rport remote local` | `HdcClient::rport` | `blocking::HdcClient::rport` | `HdcClient.rport` | Forward unit | Usually | High |
| `install` | `HdcClient::install` | `blocking::HdcClient::install` | `HdcClient.install` | Options unit, doc compile | Yes | High |
| `uninstall` | `HdcClient::uninstall` | `blocking::HdcClient::uninstall` | `HdcClient.uninstall` | Options unit, ignored integration | Yes | High |
| `start`, `kill`, `keygen` | Not exposed | Not exposed | Not exposed | Not covered | No | Conditional |
| `update`, `flash`, `erase`, `format` | Not exposed | Not exposed | Not exposed | Not covered | Yes | Conditional |
| `alive`, `spawn-sub`, `killall-sub` | Not exposed | Not exposed | Not exposed | Not covered | No | Low |

Run the local regression suite without a device:

```sh
cargo test --workspace --all-features
```

Device acceptance tests live in `hdc-rs/tests/real_device_test.rs` and are ignored
by default. Configure an explicit device and run selected cases serially. See
[the v0.2.0 review and validation notes](v0.2.0-validation-review.md) for scope,
prerequisites, and the distinction between offline and device evidence.
The legacy Rust files in the workspace root `tests/` directory are not Cargo
test targets because the root manifest is a virtual workspace.
