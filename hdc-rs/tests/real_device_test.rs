//! Real device validation test suite for hdc-rs v0.2.0
//!
//! All tests in this module require a running HDC server and a connected HarmonyOS device.
//! Environment variables used:
//! - HDC_TEST_SERVER_ADDR: HDC server address (default: "127.0.0.1:8710")
//! - HDC_TEST_DEVICE_ID: Target device ID (required for ignored real-device tests)
//! - HDC_TEST_REMOTE_DIR: Remote directory for file transfers (default: "/data/local/tmp/hdc-rs-v020-real")
//! - HDC_TEST_HAP: Path to a disposable test .hap file (required for app lifecycle tests)
//! - HDC_TEST_BUNDLE: Bundle name belonging to HDC_TEST_HAP (required for app/JDWP tests)
//! - HDC_TEST_NC: Device-side `nc` command (default: "nc", required for port data-plane tests)
//! - HDC_TEST_UPLOADED_HELPER: Explicitly test-owned absolute helper path to
//!   remove after a port data-plane test; must equal HDC_TEST_NC and be a direct
//!   child of HDC_TEST_REMOTE_DIR
//! - HDC_ALLOW_APP_CHANGES: "1" to enable install/uninstall tests
//! - HDC_ALLOW_EXISTING_TEST_BUNDLE: "1" to opt in to replacing an already-installed test bundle
//! - HDC_ALLOW_DISRUPTIVE: "1" to enable disruptive tests

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;
use std::time::Duration;

use hdc_rs::{
    FileTransferOptions, ForwardNode, HdcClient, HdcError, InstallOptions, UninstallOptions,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::info;

static INIT_LOGGER_ONCE: Once = Once::new();
static NEXT_TEST_PORT: AtomicUsize = AtomicUsize::new(0);

fn init_test_logger() {
    INIT_LOGGER_ONCE.call_once(|| {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_level(true))
            .try_init();
    });
}

/// Standard SHA-256 implementation (FIPS 180-4) without external dependencies
fn sha256_digest(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() + 8) % 64 != 0 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_val
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }

    format!(
        "{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}",
        h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]
    )
}

/// Quote one argument for the device-side POSIX shell used by `shell`.
///
/// Paths and package names in this test file come from the environment or are
/// assembled from it.  Keeping all of those values as one quoted argument
/// prevents an accidental space or shell metacharacter from changing a test
/// command.  Newlines and NULs are rejected because they cannot be represented
/// safely in a shell argument sent over the HDC command channel.
fn shell_quote(value: &str) -> String {
    assert!(
        !value
            .bytes()
            .any(|byte| matches!(byte, b'\r' | b'\n' | b'\0')),
        "shell argument contains a control character"
    );
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn ensure_command_succeeded(label: &str, response: &str) -> Result<(), String> {
    if response.trim_start().starts_with("[Fail]") {
        Err(format!("{label} returned a failure response: {response}"))
    } else {
        Ok(())
    }
}

fn ensure_remote_shell_succeeded(label: &str, response: &str) -> Result<(), String> {
    let lower = response.to_ascii_lowercase();
    if lower.contains("[fail]") || lower.contains("command not found") {
        Err(format!("{label} failed on device: {response}"))
    } else {
        Ok(())
    }
}

fn shell_command_with_status(command: &str, marker: &str) -> String {
    format!("{command}; rc=$?; printf '\\n{marker}%s\\n' \"$rc\"")
}

fn checked_shell_output<'a>(
    label: &str,
    response: &'a str,
    marker: &str,
) -> Result<&'a str, String> {
    let (output, status) = shell_output_and_status(response, marker)?;
    if status != 0 {
        return Err(format!("{label} failed (status {status}): {output}"));
    }
    ensure_remote_shell_succeeded(label, output)?;
    Ok(output)
}

fn parse_pidof_result(output: &str, status: i32) -> Result<String, String> {
    let trimmed = output.trim();
    if status == 0 {
        if trimmed.is_empty()
            || !trimmed
                .split_whitespace()
                .all(|pid| !pid.is_empty() && pid.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(format!(
                "pidof reported success but did not return numeric PID(s): {output}"
            ));
        }
        return Ok(trimmed.to_string());
    }

    if status == 1 && trimmed.is_empty() {
        return Ok(String::new());
    }

    Err(format!(
        "pidof returned unexpected status {status} or output: {output}"
    ))
}

fn shell_output_and_status<'a>(response: &'a str, marker: &str) -> Result<(&'a str, i32), String> {
    let (output, status) = response
        .rsplit_once(marker)
        .ok_or_else(|| format!("{marker} status marker missing from shell response: {response}"))?;
    let status = status
        .trim()
        .parse::<i32>()
        .map_err(|e| format!("invalid shell status after {marker}: {e}; response: {response}"))?;
    Ok((output, status))
}

#[derive(Debug, Clone)]
struct HapMetadata {
    bundle_name: String,
    version_code: u64,
    version_name: String,
    debug: bool,
    main_element: String,
}

#[derive(Debug, Clone)]
struct InstalledBundleMetadata {
    bundle_name: String,
    version_code: u64,
    version_name: String,
    debug: bool,
}

fn read_hap_module_json(path: &std::path::Path) -> Result<String, String> {
    let path = path
        .to_str()
        .ok_or_else(|| format!("HAP path is not valid UTF-8: {}", path.display()))?;
    // HAP is a ZIP container.  The Windows HDC host toolchain used for this
    // validation provides bsdtar as `tar`.  Only module.json is streamed, so
    // the large native/resource payload is not extracted to disk.
    let output = Command::new("tar")
        .args(["-xOf", path, "module.json"])
        .output()
        .map_err(|error| format!("reading module.json from HAP with tar failed: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "reading module.json from HAP failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("module.json from HAP is not UTF-8: {error}"))
}

fn read_hap_metadata(path: &std::path::Path) -> Result<HapMetadata, String> {
    let module_json = read_hap_module_json(path)?;
    let root: serde_json::Value = serde_json::from_str(&module_json)
        .map_err(|error| format!("parsing HAP module.json failed: {error}"))?;
    let app = root
        .get("app")
        .ok_or_else(|| "HAP module.json has no app object".to_string())?;
    let module = root
        .get("module")
        .ok_or_else(|| "HAP module.json has no module object".to_string())?;
    let version_code = app
        .get("versionCode")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "HAP module.json app.versionCode is missing or not numeric".to_string())?;
    let debug = app
        .get("debug")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| "HAP module.json app.debug is missing or not boolean".to_string())?;
    Ok(HapMetadata {
        bundle_name: app
            .get("bundleName")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "HAP module.json app.bundleName is missing or not a string".to_string())?
            .to_string(),
        version_code,
        version_name: app
            .get("versionName")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                "HAP module.json app.versionName is missing or not a string".to_string()
            })?
            .to_string(),
        debug,
        main_element: module
            .get("mainElement")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                "HAP module.json module.mainElement is missing or not a string".to_string()
            })?
            .to_string(),
    })
}

fn parse_installed_bundle_metadata(dump: &str) -> Result<InstalledBundleMetadata, String> {
    let json_start = dump
        .find('{')
        .ok_or_else(|| format!("bundle-manager response has no JSON object: {dump}"))?;
    let root: serde_json::Value = serde_json::from_str(&dump[json_start..]).map_err(|error| {
        format!("parsing bundle-manager JSON failed: {error}; response: {dump}")
    })?;
    let version_code = root
        .get("versionCode")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            "bundle-manager JSON root.versionCode is missing or not numeric".to_string()
        })?;
    let version_name = root
        .get("versionName")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            "bundle-manager JSON root.versionName is missing or not a string".to_string()
        })?
        .to_string();
    let debug = root
        .get("applicationInfo")
        .and_then(|application| application.get("debug"))
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| {
            "bundle-manager JSON applicationInfo.debug is missing or not boolean".to_string()
        })?;
    Ok(InstalledBundleMetadata {
        bundle_name: root
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "bundle-manager JSON root.name is missing or not a string".to_string())?
            .to_string(),
        version_code,
        version_name,
        debug,
    })
}

fn all_bundles_contains(dump: &str, bundle_name: &str) -> bool {
    dump.lines().any(|line| line.trim() == bundle_name)
}

fn next_device_port() -> u16 {
    // Keep test ports in the unprivileged range.  The atomic offset prevents
    // concurrent ignored tests in this process from choosing the same port.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before UNIX_EPOCH")
        .as_millis() as usize;
    let offset = NEXT_TEST_PORT.fetch_add(1, Ordering::Relaxed);
    40_000 + ((now.wrapping_add(offset)) % 20_000) as u16
}

fn local_mtime_seconds(path: &std::path::Path) -> Result<i64, String> {
    let modified = fs::metadata(path)
        .map_err(|e| format!("reading metadata for {}: {e}", path.display()))?
        .modified()
        .map_err(|e| format!("reading mtime for {}: {e}", path.display()))?;
    let seconds = modified
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("mtime for {} is before UNIX_EPOCH: {e}", path.display()))?
        .as_secs();
    i64::try_from(seconds).map_err(|e| format!("mtime for {} is too large: {e}", path.display()))
}

fn parse_remote_mtime(response: &str) -> Result<i64, String> {
    response
        .trim()
        .parse::<i64>()
        .map_err(|e| format!("stat did not return one numeric mtime ({e}): {response}"))
}

async fn read_exact_with_timeout(
    stream: &mut TcpStream,
    expected: &[u8],
    label: &str,
) -> Result<(), String> {
    let mut actual = vec![0u8; expected.len()];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut actual))
        .await
        .map_err(|_| format!("timed out waiting for {label}"))?
        .map_err(|e| format!("reading {label}: {e}"))?;
    if actual != expected {
        return Err(format!(
            "{label} mismatch: expected {:?}, got {:?}",
            expected, actual
        ));
    }
    Ok(())
}

async fn write_all_with_timeout(
    stream: &mut TcpStream,
    payload: &[u8],
    label: &str,
) -> Result<(), String> {
    tokio::time::timeout(Duration::from_secs(5), stream.write_all(payload))
        .await
        .map_err(|_| format!("timed out sending {label}"))?
        .map_err(|e| format!("sending {label}: {e}"))
}

fn configured_nc_command() -> String {
    let command = std::env::var("HDC_TEST_NC").unwrap_or_else(|_| "nc".to_string());
    assert!(
        !command.trim().is_empty()
            && !command
                .bytes()
                .any(|byte| matches!(byte, b'\r' | b'\n' | b'\0')),
        "HDC_TEST_NC must be a non-empty device-side command without control characters"
    );
    command
}

fn device_nc_service_command(
    nc_command: &str,
    port: u16,
    fifo_path: &str,
    pid_path: &str,
    error_path: &str,
    initial_payload: &str,
) -> String {
    // The FIFO connects the helper's stdout back into the device-side TCP
    // service, so the host can verify a complete bidirectional payload.
    format!(
        "rm -f {fifo} {pid} {error}; mkfifo {fifo} 2>{error}; (printf '%s' {payload} 2>>{error}; cat {fifo} 2>>{error}) 2>>{error} | {nc} -l -p {port} > {fifo} 2>>{error} & echo $! > {pid}",
        fifo = shell_quote(fifo_path),
        pid = shell_quote(pid_path),
        error = shell_quote(error_path),
        payload = shell_quote(initial_payload),
        nc = shell_quote(nc_command),
    )
}

fn device_nc_client_command(
    nc_command: &str,
    port: u16,
    fifo_path: &str,
    pid_path: &str,
    error_path: &str,
    initial_payload: &str,
) -> String {
    // Same FIFO echo service, with the helper in client mode.  For rport the
    // device-side client connects through the reverse mapping to the host.
    format!(
        "rm -f {fifo} {pid} {error}; mkfifo {fifo} 2>{error}; (printf '%s' {payload} 2>>{error}; cat {fifo} 2>>{error}) 2>>{error} | {nc} 127.0.0.1 {port} > {fifo} 2>>{error} & echo $! > {pid}",
        fifo = shell_quote(fifo_path),
        pid = shell_quote(pid_path),
        error = shell_quote(error_path),
        payload = shell_quote(initial_payload),
        nc = shell_quote(nc_command),
    )
}

fn device_nc_cleanup_command(
    fifo_path: &str,
    pid_path: &str,
    error_path: &str,
    helper_path: Option<&str>,
) -> String {
    let mut remove_paths = format!(
        "{} {} {}",
        shell_quote(fifo_path),
        shell_quote(pid_path),
        shell_quote(error_path)
    );
    if let Some(helper_path) = helper_path {
        remove_paths.push(' ');
        remove_paths.push_str(&shell_quote(helper_path));
    }
    format!(
        "if [ -f {pid} ]; then kill \"$(cat {pid})\" 2>/dev/null || true; fi; rm -f {remove_paths}",
        pid = shell_quote(pid_path),
        remove_paths = remove_paths,
    )
}

async fn ensure_device_service_started(
    cfg: &TestConfig,
    pid_path: &str,
    error_path: &str,
    label: &str,
) -> Result<(), String> {
    tokio::time::sleep(Duration::from_millis(100)).await;
    let mut client = cfg.try_connected_client().await?;
    let response = client
        .shell(&format!(
            "printf 'PID:'; if [ -f {pid} ]; then cat {pid}; pid_rc=$?; else pid_rc=1; fi; printf '\\nERR:'; if [ -f {error} ]; then cat {error} 2>/dev/null; error_rc=$?; else error_rc=0; fi; printf '\\nHDC_RS_HELPER_PID_RC:%s\\nHDC_RS_HELPER_ERR_RC:%s\\n' \"$pid_rc\" \"$error_rc\"",
            pid = shell_quote(pid_path),
            error = shell_quote(error_path),
        ))
        .await
        .map_err(|error| format!("reading {label} startup state: {error}"))?;
    let (without_error_status, error_status) =
        shell_output_and_status(&response, "HDC_RS_HELPER_ERR_RC:")?;
    let (without_pid_status, pid_status) =
        shell_output_and_status(without_error_status, "HDC_RS_HELPER_PID_RC:")?;
    if pid_status != 0 || error_status != 0 {
        return Err(format!(
            "{label} status check failed (pid status {pid_status}, error status {error_status}): {without_pid_status}"
        ));
    }
    let (pid, error_output) = without_pid_status
        .split_once("\nERR:")
        .ok_or_else(|| format!("{label} startup state marker missing: {response}"))?;
    let pid = pid.strip_prefix("PID:").unwrap_or(pid).trim();
    if pid.is_empty() || !pid.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!(
            "{label} did not record a numeric helper PID: {response}"
        ));
    }
    let error_output = error_output.trim();
    if !error_output.is_empty() {
        return Err(format!("{label} reported startup error: {error_output}"));
    }
    Ok(())
}

/// Remove local files/directories even when an assertion or device operation
/// fails.  All paths are generated in the OS temporary directory and are
/// registered only after their names have been chosen by this test.
struct LocalCleanup(Vec<(PathBuf, bool)>);

impl LocalCleanup {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn file(mut self, path: PathBuf) -> Self {
        self.0.push((path, false));
        self
    }

    fn dir(mut self, path: PathBuf) -> Self {
        self.0.push((path, true));
        self
    }
}

impl Drop for LocalCleanup {
    fn drop(&mut self) {
        for (path, is_dir) in self.0.iter().rev() {
            if *is_dir {
                let _ = fs::remove_dir_all(path);
            } else {
                let _ = fs::remove_file(path);
            }
        }
    }
}

#[derive(Clone)]
struct TestConfig {
    server_addr: String,
    device_id: String,
    remote_dir: String,
    hap_path: Option<PathBuf>,
    bundle_name: Option<String>,
    uploaded_helper_path: Option<String>,
    allow_app_changes: bool,
    allow_existing_test_bundle: bool,
    allow_disruptive: bool,
}

impl TestConfig {
    fn load() -> Self {
        init_test_logger();
        let server_addr =
            std::env::var("HDC_TEST_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8710".to_string());
        let device_id = std::env::var("HDC_TEST_DEVICE_ID")
            .unwrap_or_else(|_| {
                panic!("HDC_TEST_DEVICE_ID is required when running an ignored real-device test; refusing to use a personal-device default")
            });
        assert!(
            !device_id.trim().is_empty(),
            "HDC_TEST_DEVICE_ID must not be empty"
        );
        let remote_dir = std::env::var("HDC_TEST_REMOTE_DIR")
            .unwrap_or_else(|_| "/data/local/tmp/hdc-rs-v020-real".to_string());
        let hap_path = std::env::var("HDC_TEST_HAP").ok().map(PathBuf::from);
        let bundle_name = std::env::var("HDC_TEST_BUNDLE").ok();
        let uploaded_helper_path = std::env::var("HDC_TEST_UPLOADED_HELPER").ok();
        let allow_app_changes = std::env::var("HDC_ALLOW_APP_CHANGES")
            .map(|v| v == "1")
            .unwrap_or(false);
        let allow_existing_test_bundle = std::env::var("HDC_ALLOW_EXISTING_TEST_BUNDLE")
            .map(|v| v == "1")
            .unwrap_or(false);
        let allow_disruptive = std::env::var("HDC_ALLOW_DISRUPTIVE")
            .map(|v| v == "1")
            .unwrap_or(false);

        Self {
            server_addr,
            device_id,
            remote_dir,
            hap_path,
            bundle_name,
            uploaded_helper_path,
            allow_app_changes,
            allow_existing_test_bundle,
            allow_disruptive,
        }
    }

    fn required_bundle_name(&self) -> &str {
        let bundle = self.bundle_name.as_deref().unwrap_or_else(|| {
            panic!("HDC_TEST_BUNDLE is required for this test; provide the bundle belonging to the disposable HDC_TEST_HAP")
        });
        assert!(
            !bundle.is_empty()
                && bundle.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                }),
            "HDC_TEST_BUNDLE must be a valid package name"
        );
        bundle
    }

    fn required_hap_path(&self) -> &std::path::Path {
        let hap = self.hap_path.as_deref().unwrap_or_else(|| {
            panic!("HDC_TEST_HAP is required for the app lifecycle test; provide a disposable .hap")
        });
        assert!(
            hap.is_file(),
            "HDC_TEST_HAP does not point to a regular file: {}",
            hap.display()
        );
        hap
    }

    fn owned_helper_path<'a>(&'a self, nc_command: &str) -> Result<Option<&'a str>, String> {
        let Some(helper_path) = self.uploaded_helper_path.as_deref() else {
            return Ok(None);
        };
        if helper_path != nc_command {
            return Err(format!(
                "HDC_TEST_UPLOADED_HELPER must exactly equal HDC_TEST_NC ({helper_path:?} != {nc_command:?})"
            ));
        }
        if !helper_path.starts_with('/')
            || helper_path
                .bytes()
                .any(|byte| matches!(byte, b'\r' | b'\n' | b'\0'))
        {
            return Err(
                "HDC_TEST_UPLOADED_HELPER must be an absolute device path without control characters"
                    .to_string(),
            );
        }
        let remote_dir = self.remote_dir.trim_end_matches('/');
        let prefix = format!("{remote_dir}/");
        let suffix = helper_path.strip_prefix(&prefix).ok_or_else(|| {
            format!(
                "HDC_TEST_UPLOADED_HELPER must be a direct child of HDC_TEST_REMOTE_DIR ({helper_path:?} not under {remote_dir:?})"
            )
        })?;
        if remote_dir.is_empty() || remote_dir == "/" || suffix.is_empty() || suffix.contains('/') {
            return Err(
                "HDC_TEST_UPLOADED_HELPER must be a direct child of a non-root HDC_TEST_REMOTE_DIR"
                    .to_string(),
            );
        }
        Ok(Some(helper_path))
    }

    async fn new_client(&self) -> HdcClient {
        HdcClient::connect(&self.server_addr)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to connect to HDC server at {}: {}",
                    self.server_addr, e
                )
            })
    }

    async fn new_connected_client(&self) -> HdcClient {
        let mut client = self.new_client().await;
        client
            .connect_device(&self.device_id)
            .await
            .unwrap_or_else(|e| panic!("Failed to connect to device {}: {}", self.device_id, e));
        client
    }

    async fn try_connected_client(&self) -> Result<HdcClient, String> {
        let mut client = HdcClient::connect(&self.server_addr).await.map_err(|e| {
            format!(
                "failed to connect to HDC server at {}: {e}",
                self.server_addr
            )
        })?;
        client
            .connect_device(&self.device_id)
            .await
            .map_err(|e| format!("failed to connect to device {}: {e}", self.device_id))?;
        Ok(client)
    }
}

// -----------------------------------------------------------------------------
// P0 C1: Server-only one-shots and lifecycle
// -----------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_server_one_shots_and_disconnect() {
    let cfg = TestConfig::load();
    info!("Running C1: server one-shots and disconnect verification");

    // 1. check_server
    let mut client = cfg.new_client().await;
    let resp = client.check_server().await.expect("check_server failed");
    assert!(
        !resp.is_empty(),
        "check_server response should not be empty"
    );
    assert!(
        !resp.as_bytes().starts_with(&[0x00, 0x00]),
        "protocol control bytes should not leak"
    );
    assert!(
        !client.is_connected(),
        "client must disconnect after check_server"
    );
    assert!(matches!(
        client.read_response().await,
        Err(HdcError::NotConnected)
    ));

    // 2. version
    let mut client = cfg.new_client().await;
    let ver = client.version().await.expect("version failed");
    assert!(
        ver.contains("Ver:"),
        "version should contain Ver:, got: {}",
        ver
    );
    assert!(
        !client.is_connected(),
        "client must disconnect after version"
    );
    assert!(matches!(
        client.read_response().await,
        Err(HdcError::NotConnected)
    ));

    // 3. help
    let mut client = cfg.new_client().await;
    let help_normal = client.help(false).await.expect("help(false) failed");
    assert!(
        help_normal.contains("list targets"),
        "help should mention list targets"
    );
    assert!(!client.is_connected());

    let mut client = cfg.new_client().await;
    let help_verbose = client.help(true).await.expect("help(true) failed");
    assert!(
        help_verbose.len() >= help_normal.len(),
        "verbose help should be >= standard help"
    );
    assert!(!client.is_connected());

    // 4. discover
    let mut client = cfg.new_client().await;
    let disc = client.discover().await.expect("discover failed");
    info!("discover output: {}", disc);
    assert!(!client.is_connected());

    // 5. list_targets
    let mut client = cfg.new_client().await;
    let targets = client.list_targets().await.expect("list_targets failed");
    assert!(
        targets.iter().any(|t| t == &cfg.device_id),
        "targets list must contain target {}, got: {:?}",
        cfg.device_id,
        targets
    );
    assert!(
        !targets.iter().any(|t| t.trim().is_empty()),
        "targets should not contain empty lines"
    );
    assert!(!client.is_connected());

    // 6. list_targets_verbose
    let mut client = cfg.new_client().await;
    let targets_v = client
        .list_targets_verbose()
        .await
        .expect("list_targets_verbose failed");
    assert!(
        targets_v.contains(&cfg.device_id),
        "verbose list must contain device ID"
    );
    assert!(!client.is_connected());

    // 7. check_device
    let mut client = cfg.new_client().await;
    let chk = client
        .check_device(Some(&cfg.device_id))
        .await
        .expect("check_device failed");
    assert!(!chk.is_empty());
    assert!(!client.is_connected());

    // 8. connect_any
    let mut client = cfg.new_client().await;
    let any_dev = client.connect_any().await.expect("connect_any failed");
    assert!(!any_dev.is_empty());
    assert!(!client.is_connected());
}

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_wait_returns_exact_target() {
    let cfg = TestConfig::load();
    let mut client = cfg.new_client().await;
    let target = tokio::time::timeout(Duration::from_secs(15), client.wait_for_device())
        .await
        .expect("wait_for_device timed out")
        .expect("wait_for_device returned error");
    assert_eq!(
        target, cfg.device_id,
        "wait_for_device returned wrong target"
    );
    assert!(
        !client.is_connected(),
        "client must be disconnected after wait_for_device"
    );
}

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_monitor_stops_on_callback() {
    let cfg = TestConfig::load();
    let mut client = cfg.new_client().await;
    let called = AtomicUsize::new(0);
    let dev_id = cfg.device_id.clone();

    let res = tokio::time::timeout(
        Duration::from_secs(10),
        client.monitor_devices(Duration::from_secs(1), |devices| {
            called.fetch_add(1, Ordering::SeqCst);
            assert!(
                devices.iter().any(|d| d == &dev_id),
                "monitor received devices: {:?}",
                devices
            );
            false // stop on first callback
        }),
    )
    .await;

    assert!(
        res.is_ok(),
        "monitor_devices did not stop on false callback"
    );
    assert_eq!(
        called.load(Ordering::SeqCst),
        1,
        "monitor callback must be called exactly once when it stops the stream"
    );
    assert!(
        !client.is_connected(),
        "client must be disconnected after monitor"
    );

    // verify new client connects normally
    let mut next_client = cfg.new_client().await;
    assert!(next_client.version().await.is_ok());
}

// -----------------------------------------------------------------------------
// P0 C2: Device selection, shell, multi-frame, raw prefix
// -----------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_shell_reconnects_selected_target() {
    let cfg = TestConfig::load();
    let mut client = cfg.new_connected_client().await;

    let out1 = client
        .shell("echo SHELL_FIRST_TOKEN_12345")
        .await
        .expect("first shell failed");
    assert!(out1.contains("SHELL_FIRST_TOKEN_12345"));
    assert!(
        client.is_connected(),
        "client must remain connected after shell"
    );

    let out2 = client
        .shell("echo SHELL_SECOND_TOKEN_67890")
        .await
        .expect("second shell failed");
    assert!(out2.contains("SHELL_SECOND_TOKEN_67890"));
    assert!(
        client.is_connected(),
        "client must remain connected after second shell"
    );
}

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_shell_preserves_raw_prefix_and_multiframe_tail() {
    let cfg = TestConfig::load();
    let run_id = format!(
        "raw-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let remote_file = format!("{}/raw_{}.bin", cfg.remote_dir, run_id);
    let local_file = std::env::temp_dir().join(format!("hdc_raw_{}.bin", run_id));
    let _local_cleanup = LocalCleanup::new().file(local_file.clone());

    // Keep the path/configuration outside the task so cleanup can run even if
    // any assertion below panics and Tokio returns a JoinError.
    let body_cfg = cfg.clone();
    let body_remote_file = remote_file.clone();
    let body_local_file = local_file.clone();
    let body_run_id = run_id.clone();
    let body = tokio::spawn(async move {
        // Ensure remote dir
        let mut setup_client = body_cfg.new_connected_client().await;
        let setup = setup_client
            .shell(&shell_command_with_status(
                &format!("mkdir -p {}", shell_quote(&body_cfg.remote_dir)),
                "HDC_RS_SETUP_RC:",
            ))
            .await
            .expect("failed to create remote test directory");
        checked_shell_output("mkdir", &setup, "HDC_RS_SETUP_RC:")
            .expect("failed to create remote test directory");

        // Data starts with [0x02, 0x00], contains >260 KiB ASCII, ends with unique token
        let mut test_bytes = vec![0x02, 0x00];
        let block = b"HDC_RS_MULTIFRAME_ASCII_CHUNK_0123456789\n";
        while test_bytes.len() < 260 * 1024 {
            test_bytes.extend_from_slice(block);
        }
        let tail_token = format!("__HDC_TAIL_TOKEN_{}__\n", body_run_id);
        test_bytes.extend_from_slice(tail_token.as_bytes());

        fs::write(&body_local_file, &test_bytes).unwrap();

        // Send file to device
        let mut send_client = body_cfg.new_connected_client().await;
        send_client
            .file_send(
                body_local_file.to_str().unwrap(),
                &body_remote_file,
                Default::default(),
            )
            .await
            .expect("file_send failed");

        // Cat file via shell
        let mut shell_client = body_cfg.new_connected_client().await;
        let output = shell_client
            .shell(&format!("cat {}", shell_quote(&body_remote_file)))
            .await
            .expect("shell cat failed");

        let out_bytes = output.as_bytes();
        assert_eq!(
            out_bytes.len(),
            test_bytes.len(),
            "cat output length mismatch: expected {}, got {}",
            test_bytes.len(),
            out_bytes.len()
        );
        assert_eq!(
            &out_bytes[..2],
            &[0x02, 0x00],
            "[0x02, 0x00] prefix must be preserved, not treated as close frame"
        );
        assert!(
            output.ends_with(&tail_token),
            "tail token must be preserved without truncation"
        );
    });

    let body_result = body.await;
    let cleanup_cfg = cfg.clone();
    let cleanup_remote_file = remote_file.clone();
    let cleanup: Result<(), String> = async move {
        let mut clean_client = cleanup_cfg.try_connected_client().await?;
        let response = clean_client
            .shell(&shell_command_with_status(
                &format!("rm -f {}", shell_quote(&cleanup_remote_file)),
                "HDC_RS_CLEANUP_RC:",
            ))
            .await
            .map_err(|e| format!("failed to clean up remote raw test file: {e}"))?;
        checked_shell_output("raw test cleanup", &response, "HDC_RS_CLEANUP_RC:").map(|_| ())
    }
    .await;

    match (body_result, cleanup) {
        (Ok(()), Ok(())) => {}
        (body_result, cleanup_result) => {
            let body_error = body_result
                .err()
                .map(|error| format!("raw test body failed: {error}"));
            let cleanup_error = cleanup_result.err();
            let details = body_error
                .into_iter()
                .chain(cleanup_error)
                .collect::<Vec<_>>();
            panic!("raw file acceptance failed: {}", details.join("; "));
        }
    }
}

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_target_command_drains_channel() {
    let cfg = TestConfig::load();
    let mut client = cfg.new_client().await;
    let out = client
        .target_command(&cfg.device_id, "shell echo TARGET_CMD_DRAIN_TOKEN")
        .await
        .expect("target_command failed");
    assert!(out.contains("TARGET_CMD_DRAIN_TOKEN"));
    assert!(
        !client.is_connected(),
        "client must be disconnected after target_command"
    );
    assert!(matches!(
        client.read_response().await,
        Err(HdcError::NotConnected)
    ));
}

// -----------------------------------------------------------------------------
// P0 C3: File transfer
// -----------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_file_roundtrip_default() {
    let cfg = TestConfig::load();
    let run_id = format!(
        "fdef-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let remote_file = format!("{}/test_{}.bin", cfg.remote_dir, run_id);
    let local_src = std::env::temp_dir().join(format!("hdc_src_{}.bin", run_id));
    let local_dst = std::env::temp_dir().join(format!("hdc_dst_{}.bin", run_id));
    let _local_cleanup = LocalCleanup::new()
        .file(local_src.clone())
        .file(local_dst.clone());

    let body_cfg = cfg.clone();
    let body_remote_file = remote_file.clone();
    let body_local_src = local_src.clone();
    let body_local_dst = local_dst.clone();
    let body = tokio::spawn(async move {
        // 64 KiB pseudo-random binary data
        let mut data = vec![0u8; 64 * 1024];
        for (i, b) in data.iter_mut().enumerate() {
            *b = ((i * 37 + 11) & 0xFF) as u8;
        }
        fs::write(&body_local_src, &data).unwrap();
        let src_hash = sha256_digest(&data);

        let mut setup_client = body_cfg.new_connected_client().await;
        let setup = setup_client
            .shell(&shell_command_with_status(
                &format!("mkdir -p {}", shell_quote(&body_cfg.remote_dir)),
                "HDC_RS_SETUP_RC:",
            ))
            .await;
        checked_shell_output(
            "mkdir",
            &setup.expect("failed to create remote test directory"),
            "HDC_RS_SETUP_RC:",
        )
        .expect("failed to create remote test directory");

        // Send
        let mut send_client = body_cfg.new_connected_client().await;
        let s_res = send_client
            .file_send(
                body_local_src.to_str().unwrap(),
                &body_remote_file,
                Default::default(),
            )
            .await
            .expect("file_send failed");
        assert!(!s_res.starts_with("[Fail]"), "file_send error: {}", s_res);
        assert!(!send_client.is_connected());

        // Recv
        let mut recv_client = body_cfg.new_connected_client().await;
        let r_res = recv_client
            .file_recv(
                &body_remote_file,
                body_local_dst.to_str().unwrap(),
                Default::default(),
            )
            .await
            .expect("file_recv failed");
        assert!(!r_res.starts_with("[Fail]"), "file_recv error: {}", r_res);
        assert!(!recv_client.is_connected());

        // Verify local dst hash
        let dst_data = fs::read(&body_local_dst).unwrap();
        let dst_hash = sha256_digest(&dst_data);
        assert_eq!(src_hash, dst_hash, "SHA-256 mismatch after file roundtrip");

        // Verify remote hash via device sha256sum
        let mut check_client = body_cfg.new_connected_client().await;
        let remote_sum = check_client
            .shell(&format!("sha256sum {}", shell_quote(&body_remote_file)))
            .await
            .unwrap();
        assert!(
            remote_sum.to_lowercase().contains(&src_hash),
            "remote sha256sum mismatch: {}",
            remote_sum
        );
    });

    let body_result = body.await;
    let cleanup_cfg = cfg.clone();
    let cleanup_remote_file = remote_file.clone();
    let cleanup: Result<(), String> = async move {
        let mut clean_client = cleanup_cfg.try_connected_client().await?;
        let response = clean_client
            .shell(&shell_command_with_status(
                &format!("rm -f {}", shell_quote(&cleanup_remote_file)),
                "HDC_RS_CLEANUP_RC:",
            ))
            .await
            .map_err(|e| format!("failed to clean up remote roundtrip file: {e}"))?;
        checked_shell_output("roundtrip cleanup", &response, "HDC_RS_CLEANUP_RC:").map(|_| ())
    }
    .await;

    match (body_result, cleanup) {
        (Ok(()), Ok(())) => {}
        (body_result, cleanup_result) => {
            let body_error = body_result
                .err()
                .map(|error| format!("roundtrip test body failed: {error}"));
            let cleanup_error = cleanup_result.err();
            let details = body_error
                .into_iter()
                .chain(cleanup_error)
                .collect::<Vec<_>>();
            panic!("roundtrip file acceptance failed: {}", details.join("; "));
        }
    }
}

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_file_roundtrip_compressed_and_spaced_paths() {
    let cfg = TestConfig::load();
    let run_id = format!(
        "fspace-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let remote_file = format!("{}/remote file with space {}.bin", cfg.remote_dir, run_id);
    let local_dir = std::env::temp_dir().join(format!("hdc spaced dir {}", run_id));
    let _local_cleanup = LocalCleanup::new().dir(local_dir.clone());

    let local_src = local_dir.join("source space file.bin");
    let local_dst = local_dir.join("dest space file.bin");

    let body_cfg = cfg.clone();
    let body_remote_file = remote_file.clone();
    let body_local_dir = local_dir.clone();
    let body_local_src = local_src.clone();
    let body_local_dst = local_dst.clone();
    let body = tokio::spawn(async move {
        fs::create_dir_all(&body_local_dir).unwrap();

        // 2 MiB deterministic binary data
        let mut data = vec![0u8; 2 * 1024 * 1024];
        for (i, b) in data.iter_mut().enumerate() {
            *b = ((i.wrapping_mul(73)).wrapping_add(19)) as u8;
        }
        fs::write(&body_local_src, &data).unwrap();
        let src_hash = sha256_digest(&data);

        let mut setup_client = body_cfg.new_connected_client().await;
        let setup = setup_client
            .shell(&shell_command_with_status(
                &format!("mkdir -p {}", shell_quote(&body_cfg.remote_dir)),
                "HDC_RS_SETUP_RC:",
            ))
            .await;
        checked_shell_output(
            "mkdir",
            &setup.expect("failed to create remote test directory"),
            "HDC_RS_SETUP_RC:",
        )
        .expect("failed to create remote test directory");

        // Send with compress(true)
        let opts = FileTransferOptions::new().compress(true);
        let mut send_client = body_cfg.new_connected_client().await;
        let s_res = send_client
            .file_send(
                body_local_src.to_str().unwrap(),
                &body_remote_file,
                opts.clone(),
            )
            .await
            .expect("compressed file_send failed");
        assert!(!s_res.starts_with("[Fail]"));
        assert!(!send_client.is_connected());

        // Recv with compress(true)
        let mut recv_client = body_cfg.new_connected_client().await;
        let r_res = recv_client
            .file_recv(&body_remote_file, body_local_dst.to_str().unwrap(), opts)
            .await
            .expect("compressed file_recv failed");
        assert!(!r_res.starts_with("[Fail]"));
        assert!(!recv_client.is_connected());

        // Verify SHA-256
        let dst_data = fs::read(&body_local_dst).unwrap();
        let dst_hash = sha256_digest(&dst_data);
        assert_eq!(
            src_hash, dst_hash,
            "SHA-256 mismatch for compressed transfer with spaced paths"
        );

        // Negative test 1: invalid empty path returns Err locally
        let mut empty_client = body_cfg.new_connected_client().await;
        let empty_res = empty_client
            .file_send("", &body_remote_file, Default::default())
            .await;
        assert!(empty_res.is_err(), "sending empty path must return Err");

        // Negative test 2: non-existent file returns [Fail] from server
        let mut bad_client = body_cfg.new_connected_client().await;
        let missing_file = body_local_dir.join("missing_file.bin");
        let bad_res = bad_client
            .file_send(
                missing_file.to_str().unwrap(),
                &body_remote_file,
                Default::default(),
            )
            .await;
        match bad_res {
            Ok(msg) => assert!(
                msg.starts_with("[Fail]"),
                "expected [Fail] response for non-existent file, got: {}",
                msg
            ),
            Err(_) => {} // Also acceptable if client rejects
        }
    });

    let body_result = body.await;
    let cleanup_cfg = cfg.clone();
    let cleanup_remote_file = remote_file.clone();
    let cleanup: Result<(), String> = async move {
        let mut clean_client = cleanup_cfg.try_connected_client().await?;
        let response = clean_client
            .shell(&shell_command_with_status(
                &format!("rm -f {}", shell_quote(&cleanup_remote_file)),
                "HDC_RS_CLEANUP_RC:",
            ))
            .await
            .map_err(|e| format!("failed to clean up remote spaced-path file: {e}"))?;
        checked_shell_output("spaced-path cleanup", &response, "HDC_RS_CLEANUP_RC:").map(|_| ())
    }
    .await;

    match (body_result, cleanup) {
        (Ok(()), Ok(())) => {}
        (body_result, cleanup_result) => {
            let body_error = body_result
                .err()
                .map(|error| format!("spaced-path test body failed: {error}"));
            let cleanup_error = cleanup_result.err();
            let details = body_error
                .into_iter()
                .chain(cleanup_error)
                .collect::<Vec<_>>();
            panic!("spaced-path acceptance failed: {}", details.join("; "));
        }
    }
}

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_file_sync_and_timestamp_options() {
    let cfg = TestConfig::load();
    let run_id = format!(
        "fsync-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let remote_file = format!("{}/sync_{}.txt", cfg.remote_dir, run_id);
    let remote_sync_new = format!("{}/sync_new_{}.txt", cfg.remote_dir, run_id);
    let local_file = std::env::temp_dir().join(format!("hdc_sync_{}.txt", run_id));
    let _local_cleanup = LocalCleanup::new().file(local_file.clone());

    let body_cfg = cfg.clone();
    let body_remote_file = remote_file.clone();
    let body_remote_sync_new = remote_sync_new.clone();
    let body_local_file = local_file.clone();
    let body = tokio::spawn(async move {
        let expected_content = "SYNC_INITIAL_DATA_V1\n";
        fs::write(&body_local_file, expected_content).unwrap();
        // Ensure a transfer that ignores hold_timestamp cannot pass merely because
        // source and destination happen to be created in the same second.
        tokio::time::sleep(Duration::from_secs(2)).await;
        let local_mtime =
            local_mtime_seconds(&body_local_file).expect("failed to read local mtime");

        let mut setup_client = body_cfg.new_connected_client().await;
        let setup = setup_client
            .shell(&shell_command_with_status(
                &format!("mkdir -p {}", shell_quote(&body_cfg.remote_dir)),
                "HDC_RS_SETUP_RC:",
            ))
            .await;
        checked_shell_output(
            "mkdir",
            &setup.expect("failed to create remote test directory"),
            "HDC_RS_SETUP_RC:",
        )
        .expect("failed to create remote test directory");

        // 1. Send with hold_timestamp
        let mut send_client = body_cfg.new_connected_client().await;
        let ts_opts = FileTransferOptions::new().hold_timestamp(true);
        let ts_res = send_client
            .file_send(
                body_local_file.to_str().unwrap(),
                &body_remote_file,
                ts_opts,
            )
            .await
            .expect("hold_timestamp file_send failed");
        assert!(!ts_res.starts_with("[Fail]"));

        // Verify the option's actual effect at the device, at the granularity
        // supported by `stat -c %Y` (whole seconds), rather than trusting the
        // transfer response text.
        let mut ts_check_client = body_cfg.new_connected_client().await;
        let remote_mtime = ts_check_client
            .shell(&format!("stat -c %Y {}", shell_quote(&body_remote_file)))
            .await
            .expect("stat of hold_timestamp target failed");
        let remote_mtime = parse_remote_mtime(&remote_mtime).expect("remote mtime was not numeric");
        assert_eq!(
            remote_mtime, local_mtime,
            "hold_timestamp did not preserve the source mtime at one-second granularity"
        );

        // 2. Sync mode on existing target file:
        // Upstream HDC returns: "[Fail]Target file is the same date or newer"
        let mut sync_client1 = body_cfg.new_connected_client().await;
        let sync_opts = FileTransferOptions::new().sync_mode(true);
        let s_res1 = sync_client1
            .file_send(
                body_local_file.to_str().unwrap(),
                &body_remote_file,
                sync_opts.clone(),
            )
            .await
            .expect("sync_mode file_send failed");
        assert!(
            s_res1.to_ascii_lowercase().contains("same date or newer"),
            "expected 'same date or newer' message on existing file sync, got: {}",
            s_res1
        );

        // A sync response alone is not proof that no transfer happened. Keep an
        // exact content and mtime snapshot and verify both remain unchanged.
        let mut existing_check_client = body_cfg.new_connected_client().await;
        let existing_content = existing_check_client
            .shell(&format!("cat {}", shell_quote(&body_remote_file)))
            .await
            .expect("reading existing sync target failed");
        assert_eq!(
            existing_content, expected_content,
            "sync_mode changed the existing target content"
        );
        let existing_mtime = existing_check_client
            .shell(&format!("stat -c %Y {}", shell_quote(&body_remote_file)))
            .await
            .expect("stat of existing sync target failed");
        let existing_mtime =
            parse_remote_mtime(&existing_mtime).expect("existing mtime was not numeric");
        assert_eq!(
            existing_mtime, local_mtime,
            "sync_mode changed the existing target mtime"
        );

        // 3. Sync mode on non-existing target file: succeeds and transfers
        let mut sync_client2 = body_cfg.new_connected_client().await;
        let s_res2 = sync_client2
            .file_send(
                body_local_file.to_str().unwrap(),
                &body_remote_sync_new,
                sync_opts,
            )
            .await
            .expect("sync_mode on new file failed");
        assert!(
            !s_res2.starts_with("[Fail]"),
            "sync new file failed: {}",
            s_res2
        );

        // Verify content on device
        let mut check_client = body_cfg.new_connected_client().await;
        let content = check_client
            .shell(&format!("cat {}", shell_quote(&body_remote_sync_new)))
            .await
            .unwrap();
        assert_eq!(
            content, expected_content,
            "remote content mismatch for sync_mode new-file transfer"
        );
    });

    let body_result = body.await;
    let cleanup_cfg = cfg.clone();
    let cleanup_remote_file = remote_file.clone();
    let cleanup_remote_sync_new = remote_sync_new.clone();
    let cleanup: Result<(), String> = async move {
        let mut clean_client = cleanup_cfg.try_connected_client().await?;
        let response = clean_client
            .shell(&shell_command_with_status(
                &format!(
                    "rm -f {} {}",
                    shell_quote(&cleanup_remote_file),
                    shell_quote(&cleanup_remote_sync_new)
                ),
                "HDC_RS_CLEANUP_RC:",
            ))
            .await
            .map_err(|e| format!("failed to clean up sync test files: {e}"))?;
        checked_shell_output("sync cleanup", &response, "HDC_RS_CLEANUP_RC:").map(|_| ())
    }
    .await;

    match (body_result, cleanup) {
        (Ok(()), Ok(())) => {}
        (body_result, cleanup_result) => {
            let body_error = body_result
                .err()
                .map(|error| format!("sync test body failed: {error}"));
            let cleanup_error = cleanup_result.err();
            let details = body_error
                .into_iter()
                .chain(cleanup_error)
                .collect::<Vec<_>>();
            panic!("sync file acceptance failed: {}", details.join("; "));
        }
    }
}

// -----------------------------------------------------------------------------
// P0 C4: Forward and reverse port forwarding
// -----------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_fport_control_and_data_plane() {
    let cfg = TestConfig::load();
    let nc_command = configured_nc_command();
    let helper_cleanup_path = cfg
        .owned_helper_path(&nc_command)
        .unwrap_or_else(|error| panic!("invalid uploaded helper configuration: {error}"));
    let remote_port = next_device_port();
    // HDC owns the local fport listener, so reserve an ephemeral port first
    // and release it before creating the mapping.
    let probe = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("failed to reserve a local fport port");
    let local_port = probe
        .local_addr()
        .expect("failed to read local fport port")
        .port();
    drop(probe);
    let task_str = format!("tcp:{} tcp:{}", local_port, remote_port);
    let fifo_path = format!("{}/fport_fifo_{}", cfg.remote_dir, local_port);
    let pid_path = format!("{}/fport_pid_{}", cfg.remote_dir, local_port);
    let error_path = format!("{}/fport_error_{}", cfg.remote_dir, local_port);
    let device_to_host = b"HDC_FPORT_DEVICE_TO_HOST";
    let host_to_device = b"HDC_FPORT_HOST_TO_DEVICE";
    let mut forward_attempted = false;
    let mut service_attempted = false;

    let body: Result<(), String> = async {
        // A raw bidirectional data-plane check needs a device-side TCP helper.
        // Do not silently fall back to a TLS handshake or control-plane check.
        let mut prereq_client = cfg.try_connected_client().await?;
        let nc_probe = prereq_client
            .shell(&shell_command_with_status(
                &format!("command -v {}", shell_quote(&nc_command)),
                "HDC_RS_NC_RC:",
            ))
            .await
            .map_err(|e| format!("device nc prerequisite probe failed: {e}"))?;
        let nc_probe_output = checked_shell_output(
            "device nc prerequisite probe",
            &nc_probe,
            "HDC_RS_NC_RC:",
        )?;
        if nc_probe_output.trim().is_empty() {
            return Err(format!(
                "prerequisite missing: device-side `{nc_command}` is required for raw fport data-plane verification; set HDC_TEST_NC to a usable helper"
            ));
        }

        let setup = prereq_client
            .shell(&shell_command_with_status(
                &format!("mkdir -p {}", shell_quote(&cfg.remote_dir)),
                "HDC_RS_SETUP_RC:",
            ))
            .await
            .map_err(|e| format!("creating remote fport directory: {e}"))?;
        checked_shell_output("mkdir for fport", &setup, "HDC_RS_SETUP_RC:")?;

        let service_cmd = device_nc_service_command(
            &nc_command,
            remote_port,
            &fifo_path,
            &pid_path,
            &error_path,
            std::str::from_utf8(device_to_host).unwrap(),
        );
        service_attempted = true;
        let service = prereq_client
            .shell(&shell_command_with_status(&service_cmd, "HDC_RS_SERVICE_RC:"))
            .await
            .map_err(|e| format!("starting device fport TCP helper: {e}"))?;
        checked_shell_output(
            "starting device fport TCP helper",
            &service,
            "HDC_RS_SERVICE_RC:",
        )?;
        ensure_device_service_started(
            &cfg,
            &pid_path,
            &error_path,
            "device fport TCP helper",
        )
        .await?;

        // Create the mapping and prove the exact task is listed.
        forward_attempted = true;
        let mut client = cfg.try_connected_client().await?;
        let res = client
            .fport(ForwardNode::Tcp(local_port), ForwardNode::Tcp(remote_port))
            .await
            .map_err(|e| format!("fport failed: {e}"))?;
        ensure_command_succeeded("fport creation", &res)?;
        if client.is_connected() {
            return Err("fport client remained connected after terminal task".to_string());
        }

        let mut list_client = cfg.new_client().await;
        let list = list_client
            .fport_list()
            .await
            .map_err(|e| format!("fport_list failed: {e}"))?;
        if !list_client.is_connected() {
            return Err("fport_list unexpectedly disconnected its caller".to_string());
        }
        if !list.iter().any(|task| {
            task.contains(&format!("tcp:{local_port}"))
                && task.contains(&format!("tcp:{remote_port}"))
                && !task.contains("[Reverse]")
        }) {
            return Err(format!("fport task not found in fport_list: {list:?}"));
        }

        // The HDC-created local listener should connect to the device helper.
        // Read the device token, write a host token, then read the exact echo.
        let mut stream = tokio::time::timeout(
            Duration::from_secs(10),
            TcpStream::connect(("127.0.0.1", local_port)),
        )
        .await
        .map_err(|_| "timed out connecting to fport local listener".to_string())?
        .map_err(|e| format!("connecting to fport local listener: {e}"))?;
        read_exact_with_timeout(&mut stream, device_to_host, "fport device payload").await?;
        write_all_with_timeout(&mut stream, host_to_device, "fport host payload").await?;
        read_exact_with_timeout(&mut stream, host_to_device, "fport echoed host payload").await?;
        Ok(())
    }
    .await;

    // Always attempt cleanup after a mapping command was sent, including when
    // the command returned an error after creating the server-side task.
    let forward_cleanup = if forward_attempted {
        let mut rm_client = cfg.new_client().await;
        match rm_client.fport_remove(&task_str).await {
            Ok(response) => {
                async {
                    ensure_command_succeeded("fport cleanup", &response)?;
                    if !rm_client.is_connected() {
                        return Err("fport_remove unexpectedly disconnected its caller".to_string());
                    }
                    let mut list_client = cfg.new_client().await;
                    let list = list_client
                        .fport_list()
                        .await
                        .map_err(|e| format!("fport_list after cleanup failed: {e}"))?;
                    if list.iter().any(|task| {
                        task.contains(&format!("tcp:{local_port}"))
                            && task.contains(&format!("tcp:{remote_port}"))
                            && !task.contains("[Reverse]")
                    }) {
                        return Err(format!("fport task still listed after cleanup: {list:?}"));
                    }
                    Ok(())
                }
                .await
            }
            Err(e) => Err(format!("fport cleanup failed: {e}")),
        }
    } else {
        Ok(())
    };
    let service_cleanup = if service_attempted {
        let mut clean_client = cfg
            .try_connected_client()
            .await
            .map_err(|e| format!("connecting for fport helper cleanup: {e}"));
        match clean_client.as_mut() {
            Ok(client) => client
                .shell(&shell_command_with_status(
                    &device_nc_cleanup_command(
                        &fifo_path,
                        &pid_path,
                        &error_path,
                        helper_cleanup_path,
                    ),
                    "HDC_RS_CLEANUP_RC:",
                ))
                .await
                .map_err(|e| format!("fport helper cleanup failed: {e}"))
                .and_then(|response| {
                    checked_shell_output("fport helper cleanup", &response, "HDC_RS_CLEANUP_RC:")
                        .map(|_| ())
                }),
            Err(e) => Err(e.clone()),
        }
    } else {
        Ok(())
    };

    match (body, forward_cleanup, service_cleanup) {
        (Ok(()), Ok(()), Ok(())) => {}
        (body, forward_cleanup, service_cleanup) => {
            let mut details = Vec::new();
            if let Err(error) = body {
                details.push(format!("data-plane check: {error}"));
            }
            if let Err(error) = forward_cleanup {
                details.push(error);
            }
            if let Err(error) = service_cleanup {
                details.push(error);
            }
            panic!("fport acceptance failed: {}", details.join("; "));
        }
    }
}

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_rport_control_and_data_plane() {
    let cfg = TestConfig::load();
    let nc_command = configured_nc_command();
    let helper_cleanup_path = cfg
        .owned_helper_path(&nc_command)
        .unwrap_or_else(|error| panic!("invalid uploaded helper configuration: {error}"));
    let remote_port = next_device_port();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind host rport listener failed");
    let local_port = listener
        .local_addr()
        .expect("failed to read host rport listener port")
        .port();
    let task_str = format!("tcp:{} tcp:{}", remote_port, local_port);
    let fifo_path = format!("{}/rport_fifo_{}", cfg.remote_dir, remote_port);
    let pid_path = format!("{}/rport_pid_{}", cfg.remote_dir, remote_port);
    let error_path = format!("{}/rport_error_{}", cfg.remote_dir, remote_port);
    let device_to_host = b"HDC_RPORT_DEVICE_TO_HOST";
    let host_to_device = b"HDC_RPORT_HOST_TO_DEVICE";
    let mut forward_attempted = false;
    let mut service_attempted = false;

    let body: Result<(), String> = async {
        let mut client = cfg.try_connected_client().await?;
        let nc_probe = client
            .shell(&shell_command_with_status(
                &format!("command -v {}", shell_quote(&nc_command)),
                "HDC_RS_NC_RC:",
            ))
            .await
            .map_err(|e| format!("device nc prerequisite probe failed: {e}"))?;
        let nc_probe_output = checked_shell_output(
            "device nc prerequisite probe",
            &nc_probe,
            "HDC_RS_NC_RC:",
        )?;
        if nc_probe_output.trim().is_empty() {
            return Err(format!(
                "prerequisite missing: device-side `{nc_command}` is required for raw rport data-plane verification; set HDC_TEST_NC to a usable helper"
            ));
        }

        let setup = client
            .shell(&shell_command_with_status(
                &format!("mkdir -p {}", shell_quote(&cfg.remote_dir)),
                "HDC_RS_SETUP_RC:",
            ))
            .await
            .map_err(|e| format!("creating remote rport directory: {e}"))?;
        checked_shell_output("mkdir for rport", &setup, "HDC_RS_SETUP_RC:")?;

        forward_attempted = true;
        let res = client
            .rport(ForwardNode::Tcp(remote_port), ForwardNode::Tcp(local_port))
            .await
            .map_err(|e| format!("rport failed: {e}"))?;
        ensure_command_succeeded("rport creation", &res)?;
        if client.is_connected() {
            return Err("rport client remained connected after terminal task".to_string());
        }

        let mut list_client = cfg.new_client().await;
        let list = list_client
            .fport_list()
            .await
            .map_err(|e| format!("fport_list failed: {e}"))?;
        if !list_client.is_connected() {
            return Err("fport_list unexpectedly disconnected its caller".to_string());
        }
        if !list.iter().any(|task| {
            task.contains(&format!("tcp:{remote_port}"))
                && task.contains(&format!("tcp:{local_port}"))
                && task.contains("[Reverse]")
        }) {
            return Err(format!("rport task not found in fport_list: {list:?}"));
        }

        let service_cmd = device_nc_client_command(
            &nc_command,
            remote_port,
            &fifo_path,
            &pid_path,
            &error_path,
            std::str::from_utf8(device_to_host).unwrap(),
        );
        service_attempted = true;
        // rport() consumes its terminal command channel.  Start the
        // device-side helper through a fresh connected client.
        let mut service_client = cfg.try_connected_client().await?;
        let service = service_client
            .shell(&shell_command_with_status(&service_cmd, "HDC_RS_SERVICE_RC:"))
            .await
            .map_err(|e| format!("starting device rport TCP helper: {e}"))?;
        checked_shell_output(
            "starting device rport TCP helper",
            &service,
            "HDC_RS_SERVICE_RC:",
        )?;
        ensure_device_service_started(
            &cfg,
            &pid_path,
            &error_path,
            "device rport TCP helper",
        )
        .await?;

        let (mut socket, _) = tokio::time::timeout(Duration::from_secs(10), listener.accept())
            .await
            .map_err(|_| "timed out waiting for rport connection".to_string())?
            .map_err(|e| format!("accepting rport connection: {e}"))?;
        read_exact_with_timeout(&mut socket, device_to_host, "rport device payload").await?;
        write_all_with_timeout(&mut socket, host_to_device, "rport host payload").await?;
        read_exact_with_timeout(&mut socket, host_to_device, "rport echoed host payload").await?;
        Ok(())
    }
    .await;

    drop(listener);

    let forward_cleanup = if forward_attempted {
        let mut rm_client = cfg.new_client().await;
        match rm_client.fport_remove(&task_str).await {
            Ok(response) => {
                async {
                    ensure_command_succeeded("rport cleanup", &response)?;
                    if !rm_client.is_connected() {
                        return Err("fport_remove unexpectedly disconnected its caller".to_string());
                    }
                    let mut list_client = cfg.new_client().await;
                    let list = list_client
                        .fport_list()
                        .await
                        .map_err(|e| format!("fport_list after rport cleanup failed: {e}"))?;
                    if list.iter().any(|task| {
                        task.contains(&format!("tcp:{remote_port}"))
                            && task.contains(&format!("tcp:{local_port}"))
                            && task.contains("[Reverse]")
                    }) {
                        return Err(format!("rport task still listed after cleanup: {list:?}"));
                    }
                    Ok(())
                }
                .await
            }
            Err(e) => Err(format!("rport cleanup failed: {e}")),
        }
    } else {
        Ok(())
    };
    let service_cleanup = if service_attempted {
        let mut clean_client = cfg
            .try_connected_client()
            .await
            .map_err(|e| format!("connecting for rport helper cleanup: {e}"));
        match clean_client.as_mut() {
            Ok(client) => client
                .shell(&shell_command_with_status(
                    &device_nc_cleanup_command(
                        &fifo_path,
                        &pid_path,
                        &error_path,
                        helper_cleanup_path,
                    ),
                    "HDC_RS_CLEANUP_RC:",
                ))
                .await
                .map_err(|e| format!("rport helper cleanup failed: {e}"))
                .and_then(|response| {
                    checked_shell_output("rport helper cleanup", &response, "HDC_RS_CLEANUP_RC:")
                        .map(|_| ())
                }),
            Err(e) => Err(e.clone()),
        }
    } else {
        Ok(())
    };

    match (body, forward_cleanup, service_cleanup) {
        (Ok(()), Ok(()), Ok(())) => {}
        (body, forward_cleanup, service_cleanup) => {
            let mut details = Vec::new();
            if let Err(error) = body {
                details.push(format!("data-plane check: {error}"));
            }
            if let Err(error) = forward_cleanup {
                details.push(error);
            }
            if let Err(error) = service_cleanup {
                details.push(error);
            }
            panic!("rport acceptance failed: {}", details.join("; "));
        }
    }
}

// -----------------------------------------------------------------------------
// P0 C5: Application install, replace, and uninstall
// -----------------------------------------------------------------------------

async fn device_all_bundles(cfg: &TestConfig) -> Result<String, String> {
    let mut client = cfg.try_connected_client().await?;
    let response = client
        .shell("bm dump -a; rc=$?; printf '\\nHDC_RS_BM_RC:%s\\n' \"$rc\"")
        .await
        .map_err(|error| format!("bundle list query failed: {error}"))?;
    let (dump, status) = shell_output_and_status(&response, "HDC_RS_BM_RC:")?;
    if status != 0 {
        return Err(format!("bm dump -a failed (status {status}): {dump}"));
    }
    ensure_remote_shell_succeeded("bm dump -a", &response)?;
    Ok(dump.to_string())
}

async fn device_bundle_metadata(
    cfg: &TestConfig,
    bundle_name: &str,
) -> Result<InstalledBundleMetadata, String> {
    let mut client = cfg.try_connected_client().await?;
    let response = client
        .shell(&format!(
            "bm dump -n {}; rc=$?; printf '\\nHDC_RS_BM_RC:%s\\n' \"$rc\"",
            shell_quote(bundle_name)
        ))
        .await
        .map_err(|error| format!("bundle metadata query failed: {error}"))?;
    let (dump, status) = shell_output_and_status(&response, "HDC_RS_BM_RC:")?;
    if status != 0 {
        return Err(format!(
            "bm dump -n {bundle_name} failed (status {status}): {dump}"
        ));
    }
    ensure_remote_shell_succeeded("bm dump -n", &response)?;
    parse_installed_bundle_metadata(dump)
}

async fn device_pid(cfg: &TestConfig, bundle_name: &str) -> Result<String, String> {
    let mut client = cfg.try_connected_client().await?;
    let response = client
        .shell(&format!(
            "pidof {}; rc=$?; printf '\\nHDC_RS_PID_RC:%s\\n' \"$rc\"",
            shell_quote(bundle_name)
        ))
        .await
        .map_err(|error| format!("pidof {bundle_name} failed: {error}"))?;
    let (output, status) = shell_output_and_status(&response, "HDC_RS_PID_RC:")?;
    parse_pidof_result(output, status)
}

fn app_response_has_error_hint(response: &str) -> bool {
    let lower = response.to_ascii_lowercase();
    lower.contains("[fail]") || lower.contains("msg:error")
}

async fn uninstall_test_bundle(
    cfg: &TestConfig,
    bundle_name: &str,
    keep_data: bool,
    label: &str,
) -> Result<String, String> {
    let mut client = cfg.try_connected_client().await?;
    let response = client
        .uninstall(bundle_name, UninstallOptions::new().keep_data(keep_data))
        .await
        .map_err(|error| format!("{label} SDK call failed: {error}"))?;
    if client.is_connected() {
        return Err(format!(
            "{label} client remained connected after terminal task"
        ));
    }
    if app_response_has_error_hint(&response) {
        return Err(format!(
            "{label} returned an error response; package state must be inspected during cleanup: {response}"
        ));
    }
    Ok(response)
}

async fn install_test_hap(
    cfg: &TestConfig,
    hap_path: &str,
    label: &str,
    options: InstallOptions,
) -> Result<String, String> {
    let mut client = cfg.try_connected_client().await?;
    let response = client
        .install(&[hap_path], options)
        .await
        .map_err(|error| format!("{label} SDK call failed: {error}"))?;
    if client.is_connected() {
        return Err(format!(
            "{label} client remained connected after terminal task"
        ));
    }
    if app_response_has_error_hint(&response) {
        return Err(format!(
            "{label} returned an error response; package state must be inspected during cleanup: {response}"
        ));
    }
    Ok(response)
}

async fn remove_test_bundle_if_present(
    cfg: &TestConfig,
    bundle_name: &str,
    keep_data: bool,
    label: &str,
) -> Result<(), String> {
    let before = device_all_bundles(cfg).await?;
    if !all_bundles_contains(&before, bundle_name) {
        return Ok(());
    }
    uninstall_test_bundle(cfg, bundle_name, keep_data, label).await?;
    let after = device_all_bundles(cfg).await?;
    if all_bundles_contains(&after, bundle_name) {
        return Err(format!(
            "{label} did not remove {bundle_name}; refusing to retry without the requested keep_data={keep_data} option"
        ));
    }
    Ok(())
}

async fn verify_test_bundle(
    cfg: &TestConfig,
    bundle_name: &str,
    hap_metadata: &HapMetadata,
    label: &str,
) -> Result<InstalledBundleMetadata, String> {
    let metadata = device_bundle_metadata(cfg, bundle_name).await?;
    if metadata.bundle_name != bundle_name {
        return Err(format!(
            "{label} returned bundle {}, expected {bundle_name}",
            metadata.bundle_name
        ));
    }
    if metadata.version_code != hap_metadata.version_code {
        return Err(format!(
            "{label} versionCode mismatch: expected {}, got {}",
            hap_metadata.version_code, metadata.version_code
        ));
    }
    if metadata.version_name != hap_metadata.version_name {
        return Err(format!(
            "{label} versionName mismatch: expected {}, got {}",
            hap_metadata.version_name, metadata.version_name
        ));
    }
    if metadata.debug != hap_metadata.debug {
        return Err(format!(
            "{label} debug mismatch: HAP={}, device={}",
            hap_metadata.debug, metadata.debug
        ));
    }
    Ok(metadata)
}

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_install_replace_and_uninstall() {
    let cfg = TestConfig::load();
    assert!(
        cfg.allow_app_changes,
        "application lifecycle test requires explicit HDC_ALLOW_APP_CHANGES=1 authorization"
    );
    let hap_path = cfg.required_hap_path();
    let bundle_name = cfg.required_bundle_name();
    let hap_path = hap_path
        .to_str()
        .unwrap_or_else(|| {
            panic!(
                "HDC_TEST_HAP path is not valid UTF-8: {}",
                hap_path.display()
            )
        })
        .to_string();

    let hap_metadata = read_hap_metadata(std::path::Path::new(&hap_path))
        .unwrap_or_else(|error| panic!("HAP module.json preflight failed: {error}"));
    if hap_metadata.bundle_name != bundle_name {
        panic!(
            "HDC_TEST_BUNDLE {} does not match HAP module.json bundleName {}",
            bundle_name, hap_metadata.bundle_name
        );
    }
    if hap_metadata.main_element != "MainAbility" {
        panic!(
            "HAP module.json mainElement {} does not match the application test's MainAbility entry point",
            hap_metadata.main_element
        );
    }

    info!(
        "Starting application lifecycle test with bundle={} versionName={} versionCode={} debug={} mainElement={}",
        bundle_name,
        hap_metadata.version_name,
        hap_metadata.version_code,
        hap_metadata.debug,
        hap_metadata.main_element
    );

    // The full bundle list is the authoritative installed/not-installed
    // baseline.  A failed bm command is never treated as an absent package.
    let baseline_dump = device_all_bundles(&cfg)
        .await
        .unwrap_or_else(|error| panic!("baseline bundle query failed: {error}"));
    let baseline_present = all_bundles_contains(&baseline_dump, bundle_name);
    let baseline_metadata = if baseline_present {
        let metadata = device_bundle_metadata(&cfg, bundle_name)
            .await
            .unwrap_or_else(|error| panic!("baseline bundle metadata query failed: {error}"));
        if metadata.bundle_name != bundle_name {
            panic!(
                "baseline metadata named {} instead of configured bundle {}",
                metadata.bundle_name, bundle_name
            );
        }
        Some(metadata)
    } else {
        None
    };

    if baseline_present && !cfg.allow_existing_test_bundle {
        panic!(
            "bundle {} is already installed; refusing to modify an existing app. Set HDC_ALLOW_EXISTING_TEST_BUNDLE=1 only after recording and accepting the baseline",
            bundle_name
        );
    }
    if let Some(metadata) = &baseline_metadata {
        if metadata.version_code != hap_metadata.version_code
            || metadata.version_name != hap_metadata.version_name
            || metadata.debug != hap_metadata.debug
        {
            panic!(
                "existing bundle baseline does not match HAP module.json: baseline={metadata:?}, HAP={hap_metadata:?}; refusing because this test cannot restore the original package"
            );
        }
        let pid = device_pid(&cfg, bundle_name)
            .await
            .unwrap_or_else(|error| panic!("baseline app running-state query failed: {error}"));
        if !pid.is_empty() {
            panic!(
                "existing bundle {} is running with PID {}; refusing to force-stop or alter the user's app state",
                bundle_name, pid
            );
        }
        info!(
            "Accepted explicitly authorized existing-package baseline: bundle={} versionName={} versionCode={} debug={}; user data is retained only through keep_data cleanup",
            metadata.bundle_name, metadata.version_name, metadata.version_code, metadata.debug
        );
    }

    let mut mutation_started = false;
    let body: Result<(), String> = async {
        mutation_started = true;
        if baseline_present {
            remove_test_bundle_if_present(
                &cfg,
                bundle_name,
                true,
                "existing-package keep_data uninstall",
            )
            .await?;
        }

        info!("Installing selected HAP: {}", hap_path);
        install_test_hap(&cfg, &hap_path, "install", InstallOptions::new()).await?;
        verify_test_bundle(&cfg, bundle_name, &hap_metadata, "install verification").await?;

        info!("Testing replace install...");
        install_test_hap(
            &cfg,
            &hap_path,
            "replace install",
            InstallOptions::new().replace(true),
        )
        .await?;
        verify_test_bundle(
            &cfg,
            bundle_name,
            &hap_metadata,
            "replace install verification",
        )
        .await?;
        Ok(())
    }
    .await;

    // An originally installed package is restored with the same selected HAP
    // and keep_data on every uninstall.  This verifies installation state and
    // version identity only; it does not claim that user data was restored.
    let cleanup = if mutation_started {
        if baseline_present {
            let removed =
                remove_test_bundle_if_present(&cfg, bundle_name, true, "existing-package cleanup")
                    .await;
            match removed {
                Ok(()) => {
                    let restored = install_test_hap(
                        &cfg,
                        &hap_path,
                        "baseline package restore",
                        InstallOptions::new(),
                    )
                    .await;
                    match restored {
                        Err(error) => Err(error),
                        Ok(_) => verify_test_bundle(
                            &cfg,
                            bundle_name,
                            &hap_metadata,
                            "baseline package restore verification",
                        )
                        .await
                        .map(|_| ()),
                    }
                }
                Err(error) => Err(error),
            }
        } else {
            remove_test_bundle_if_present(&cfg, bundle_name, false, "disposable app cleanup").await
        }
    } else {
        Ok(())
    };

    let post_cleanup = if cleanup.is_ok() {
        match device_all_bundles(&cfg).await {
            Err(error) => Err(format!("post-cleanup bundle query failed: {error}")),
            Ok(_) if baseline_present => verify_test_bundle(
                &cfg,
                bundle_name,
                &hap_metadata,
                "final baseline package verification",
            )
            .await
            .map(|_| ()),
            Ok(dump) if all_bundles_contains(&dump, bundle_name) => Err(format!(
                "bundle is still installed after cleanup: {bundle_name}"
            )),
            Ok(_) => Ok(()),
        }
    } else {
        Ok(())
    };

    match (body, cleanup, post_cleanup) {
        (Ok(()), Ok(()), Ok(())) if baseline_present => info!(
            "Application lifecycle completed; baseline package version {} restored with keep_data cleanup. User data restoration was not verified.",
            hap_metadata.version_code
        ),
        (Ok(()), Ok(()), Ok(())) => info!("Disposable app lifecycle completed and cleaned up"),
        (body, cleanup, post_cleanup) => {
            let mut details = Vec::new();
            if let Err(error) = body {
                details.push(error);
            }
            if let Err(error) = cleanup {
                details.push(error);
            }
            if let Err(error) = post_cleanup {
                details.push(error);
            }
            panic!(
                "application lifecycle acceptance failed: {}",
                details.join("; ")
            );
        }
    }
}

// -----------------------------------------------------------------------------
// P0 C6: Hilog and JDWP
// -----------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_hilog_buffered_and_stream_stop() {
    let cfg = TestConfig::load();

    // 1. Buffered hilog with -z 15
    let mut client = cfg.new_connected_client().await;
    let logs = client.hilog(Some("-z 15")).await.expect("hilog failed");
    assert!(!logs.is_empty(), "hilog output should not be empty");
    assert!(!client.is_connected(), "client must disconnect after hilog");

    // 2. hilog_stream callback stop
    let mut stream_client = cfg.new_connected_client().await;
    let count = AtomicUsize::new(0);

    let res = tokio::time::timeout(
        Duration::from_secs(10),
        stream_client.hilog_stream(None, |_line| {
            count.fetch_add(1, Ordering::SeqCst);
            false // stop after first log entry
        }),
    )
    .await;

    assert!(res.is_ok(), "hilog_stream did not terminate within timeout");
    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "hilog_stream callback must be called exactly once when it stops the stream"
    );
    assert!(
        !stream_client.is_connected(),
        "client must disconnect after hilog_stream"
    );

    // 3. Verify server connection is healthy
    let mut check_client = cfg.new_connected_client().await;
    let out = check_client.shell("echo HILOG_DRAINED_OK").await.unwrap();
    assert!(out.contains("HILOG_DRAINED_OK"));
}

#[tokio::test]
#[ignore = "requires real HDC device"]
async fn real_device_jpid_and_track_jpid() {
    let cfg = TestConfig::load();
    let bundle_name = cfg.required_bundle_name();

    // Start the explicitly selected disposable/debuggable bundle, then always
    // force-stop it below even if PID or stream assertions fail.
    let body: Result<(), String> = async {
        let mut setup_client = cfg.try_connected_client().await?;
        let start_res = setup_client
            .shell(&shell_command_with_status(
                &format!("aa start -a MainAbility -b {}", shell_quote(bundle_name)),
                "HDC_RS_APP_START_RC:",
            ))
            .await
            .map_err(|e| format!("starting {bundle_name}: {e}"))?;
        checked_shell_output("aa start", &start_res, "HDC_RS_APP_START_RC:")?;
        info!("App start: {}", start_res);
        tokio::time::sleep(Duration::from_millis(800)).await;

        let pid_response = setup_client
            .shell(&shell_command_with_status(
                &format!("pidof {}", shell_quote(bundle_name)),
                "HDC_RS_PID_RC:",
            ))
            .await
            .map_err(|e| format!("pidof {bundle_name}: {e}"))?;
        let (pid_output, pid_status) = shell_output_and_status(&pid_response, "HDC_RS_PID_RC:")?;
        let pid_str = parse_pidof_result(pid_output, pid_status)?;
        let expected_pids: Vec<&str> = pid_str.split_whitespace().collect();
        if expected_pids.is_empty() {
            return Err("pidof returned no PID for the started test app".to_string());
        }
        if !expected_pids
            .iter()
            .all(|pid| pid.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(format!("pidof returned non-numeric PID(s): {pid_str}"));
        }
        info!("Test app PID: {}", pid_str);

        // jpid()
        let mut jpid_client = cfg.try_connected_client().await?;
        let pids = jpid_client
            .jpid()
            .await
            .map_err(|e| format!("jpid failed: {e}"))?;
        if jpid_client.is_connected() {
            return Err("jpid client remained connected after terminal task".to_string());
        }
        if pids.is_empty() {
            return Err("jpid list was empty".to_string());
        }
        if let Some(pid) = pids
            .iter()
            .find(|pid| !pid.chars().all(|c| c.is_ascii_digit()))
        {
            return Err(format!("jpid returned non-numeric PID: {pid}"));
        }
        if !expected_pids
            .iter()
            .any(|expected| pids.iter().any(|actual| actual == expected))
        {
            return Err(format!(
                "jpid list did not contain a PID reported for {} ({}): {:?}",
                bundle_name, pid_str, pids
            ));
        }

        // track_jpid()
        let mut track_client = cfg.try_connected_client().await?;
        let called = AtomicUsize::new(0);
        let track_res = tokio::time::timeout(
            Duration::from_secs(10),
            track_client.track_jpid(false, false, |_line| {
                called.fetch_add(1, Ordering::SeqCst);
                false // stop on first callback
            }),
        )
        .await
        .map_err(|_| "track_jpid did not terminate within timeout".to_string())?
        .map_err(|e| format!("track_jpid failed: {e}"))?;
        let _ = track_res;
        if called.load(Ordering::SeqCst) != 1 {
            return Err(format!(
                "track_jpid callback must be called exactly once; got {}",
                called.load(Ordering::SeqCst)
            ));
        }
        if track_client.is_connected() {
            return Err("track_jpid client remained connected after terminal task".to_string());
        }
        Ok(())
    }
    .await;

    let cleanup: Result<(), String> = async {
        let mut client = cfg
            .try_connected_client()
            .await
            .map_err(|e| format!("connecting to stop test app: {e}"))?;
        let response = client
            .shell(&shell_command_with_status(
                &format!("aa force-stop {}", shell_quote(bundle_name)),
                "HDC_RS_APP_STOP_RC:",
            ))
            .await
            .map_err(|e| format!("force-stop {bundle_name} failed: {e}"))?;
        checked_shell_output("aa force-stop", &response, "HDC_RS_APP_STOP_RC:").map(|_| ())
    }
    .await;

    match (body, cleanup) {
        (Ok(()), Ok(())) => {}
        (body, cleanup) => {
            let mut details = Vec::new();
            if let Err(error) = body {
                details.push(error);
            }
            if let Err(error) = cleanup {
                details.push(error);
            }
            panic!("jpid acceptance failed: {}", details.join("; "));
        }
    }
}

// -----------------------------------------------------------------------------
// P0 C8: Rust blocking smoke test
// -----------------------------------------------------------------------------

#[cfg(feature = "blocking")]
#[test]
#[ignore = "requires real HDC device"]
fn real_device_blocking_smoke() {
    let cfg = TestConfig::load();
    info!("Running Rust blocking smoke test");

    // 1. Version
    let mut client =
        hdc_rs::blocking::HdcClient::connect(&cfg.server_addr).expect("blocking connect failed");
    let ver = client.version().expect("blocking version failed");
    assert!(ver.contains("Ver:"));

    // 2. List targets
    let mut client =
        hdc_rs::blocking::HdcClient::connect(&cfg.server_addr).expect("blocking connect failed");
    let targets = client.list_targets().expect("blocking list_targets failed");
    assert!(targets.iter().any(|t| t == &cfg.device_id));

    // 3. Connect + shell
    let mut client =
        hdc_rs::blocking::HdcClient::connect(&cfg.server_addr).expect("blocking connect failed");
    client
        .connect_device(&cfg.device_id)
        .expect("blocking connect_device failed");
    let sh1 = client.shell("echo BLOCKING_FIRST").expect("shell 1 failed");
    assert!(sh1.contains("BLOCKING_FIRST"));
    let sh2 = client
        .shell("echo BLOCKING_SECOND")
        .expect("shell 2 failed");
    assert!(sh2.contains("BLOCKING_SECOND"));

    // 4. File roundtrip
    let run_id = format!(
        "blk-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let remote_file = format!("{}/blk_{}.txt", cfg.remote_dir, run_id);
    let local_src = std::env::temp_dir().join(format!("hdc_blk_src_{}.txt", run_id));
    let local_dst = std::env::temp_dir().join(format!("hdc_blk_dst_{}.txt", run_id));
    let _local_cleanup = LocalCleanup::new()
        .file(local_src.clone())
        .file(local_dst.clone());
    fs::write(&local_src, "BLOCKING_FILE_PAYLOAD_TEST\n").unwrap();

    let mut send_client = hdc_rs::blocking::HdcClient::connect(&cfg.server_addr).unwrap();
    send_client.connect_device(&cfg.device_id).unwrap();
    send_client
        .file_send(
            local_src.to_str().unwrap(),
            &remote_file,
            Default::default(),
        )
        .expect("blocking file_send failed");

    let mut recv_client = hdc_rs::blocking::HdcClient::connect(&cfg.server_addr).unwrap();
    recv_client.connect_device(&cfg.device_id).unwrap();
    recv_client
        .file_recv(
            &remote_file,
            local_dst.to_str().unwrap(),
            Default::default(),
        )
        .expect("blocking file_recv failed");

    let content = fs::read_to_string(&local_dst).unwrap();
    assert_eq!(content, "BLOCKING_FILE_PAYLOAD_TEST\n");

    let mut clean_client = hdc_rs::blocking::HdcClient::connect(&cfg.server_addr).unwrap();
    clean_client.connect_device(&cfg.device_id).unwrap();
    let cleanup = clean_client
        .shell(&shell_command_with_status(
            &format!("rm -f {}", shell_quote(&remote_file)),
            "HDC_RS_CLEANUP_RC:",
        ))
        .expect("blocking roundtrip cleanup failed");
    checked_shell_output("blocking roundtrip cleanup", &cleanup, "HDC_RS_CLEANUP_RC:")
        .expect("blocking roundtrip cleanup failed");

    // 5. fport create & remove
    let mut fp_client = hdc_rs::blocking::HdcClient::connect(&cfg.server_addr).unwrap();
    fp_client.connect_device(&cfg.device_id).unwrap();
    let fp_res = fp_client
        .fport(ForwardNode::Tcp(28998), ForwardNode::Tcp(28999))
        .expect("blocking fport failed");
    assert!(!fp_res.starts_with("[Fail]"));

    let mut rm_client = hdc_rs::blocking::HdcClient::connect(&cfg.server_addr).unwrap();
    let rm_res = rm_client
        .fport_remove("tcp:28998 tcp:28999")
        .expect("blocking fport_remove failed");
    assert!(!rm_res.starts_with("[Fail]"));

    // 6. hilog stream stop
    let mut hl_client = hdc_rs::blocking::HdcClient::connect(&cfg.server_addr).unwrap();
    hl_client.connect_device(&cfg.device_id).unwrap();
    let count = AtomicUsize::new(0);
    hl_client
        .hilog_stream(None, |_line| {
            count.fetch_add(1, Ordering::SeqCst);
            false
        })
        .expect("blocking hilog_stream failed");
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

// -----------------------------------------------------------------------------
// P1: Disruptive commands (gated by HDC_ALLOW_DISRUPTIVE=1)
// -----------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires explicit HDC_ALLOW_DISRUPTIVE=1 authorization"]
async fn real_device_disruptive_target_controls() {
    let cfg = TestConfig::load();
    assert!(
        cfg.allow_disruptive,
        "disruptive target-control tests require explicit HDC_ALLOW_DISRUPTIVE=1 authorization"
    );
    // Only executed if authorized
}

#[cfg(test)]
mod pure_validation_tests {
    use super::parse_pidof_result;

    #[test]
    fn pidof_accepts_numeric_pids_only_on_success() {
        assert_eq!(parse_pidof_result("12345\n", 0).unwrap(), "12345");
        assert_eq!(
            parse_pidof_result("12345 67890\n", 0).unwrap(),
            "12345 67890"
        );
        assert!(parse_pidof_result("", 0).is_err());
        assert!(parse_pidof_result("123x\n", 0).is_err());
    }

    #[test]
    fn pidof_accepts_empty_output_only_for_not_found_status() {
        assert_eq!(parse_pidof_result("\n", 1).unwrap(), "");
        assert!(parse_pidof_result("pidof: not found\n", 1).is_err());
        assert!(parse_pidof_result("\n", 2).is_err());
        assert!(parse_pidof_result("\n", 127).is_err());
    }
}
