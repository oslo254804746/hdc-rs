# Changelog

## [0.2.0] - 2026-08-28

### Added

- Expanded async, blocking, and Python coverage for target management, port
  forwarding, file transfer, application management, and JDWP process tracking.
- Added command option rendering tests and validation of values that cannot be
  safely encoded. Only `-cwd` and install/file source paths support host quoting
  for spaces; daemon option values remain restricted to safe single tokens.

### Changed

- Install option/value pairs are encoded as one quoted HDC argument; `-cwd`
  remains an option followed by a separate value argument. Uninstall sends
  its package selector as separate `-n` and package arguments.
- Buffered Hilog captures for at most two seconds and ends earlier after a
  500 ms packet timeout once output arrives. Streaming Hilog preserves UTF-8
  across packets, replaces malformed bytes, and stops callbacks immediately
  when they return false. Python `hilog()` defaults its arguments to `None`.
- Installation allows up to 300 seconds between response frames.
- Corrected the documented minimum supported Rust version to 1.71, matching
  the dependency graph, and added an MSRV check to CI.
- Covered terminal/streaming task commands use a frame-aware channel drain that
  recognizes `KernelChannelClose`; raw shell output uses a multi-frame EOF drain
  so binary prefixes are preserved. These covered terminal/streaming paths
  invalidate the channel on success, EOF, error, and timeout.
- Rust CI validates the complete workspace with all features and configures
  `PYO3_PYTHON` for Python bindings.

### Validation

- Device-dependent integration tests remain ignored by default. Running them
  requires an HDC server and a connected HarmonyOS/OpenHarmony device.
- Added offline Hilog and application protocol regressions and an installed
  Python wheel regression script. Device acceptance requires an explicit
  device ID; application lifecycle tests also require explicit opt-in and a
  disposable package that is not already installed. See the
  [validation review](docs/v0.2.0-validation-review.md) for actual test results
  and the remaining hardware acceptance requirements.

### Deferred / Not included

- `bugreport` and `sideload` remain deferred pending multi-frame
  implementations and real-device validation.
- Reverse forwarding list/removal uses the unified `fport ls` and `fport rm`
  commands; dedicated reverse list/removal APIs are not included.

[0.2.0]: https://github.com/oslo254804746/hdc-rs/compare/v0.1.2...v0.2.0
