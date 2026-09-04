#!/usr/bin/env python3
"""Verify fport/rport application data through a real TLS connection.

The default ``bridge`` backend starts a TLS echo server on the host, creates a
reverse mapping from device port D to that server port B, and then creates a
forward mapping from host port A to D.  A host TLS client connects to A, so the
complete application payload travels through both fport and rport in both
directions.  This works on device images whose shell SELinux domain cannot
bind a TCP listener.  Set ``HDC_TEST_FORWARD_BACKEND=openssl`` to use the
legacy backend, which starts OpenSSL ``s_server -rev``/``s_client`` on the
device and is useful on less restricted images.

Both backends perform a real TLS handshake and exact byte-for-byte payload
checks.  The host TLS client/server trust the generated certificate explicitly
and perform hostname verification; no ``CERT_NONE`` or insecure verification
mode is used.

Run this script with the CPython 3.11 environment containing the validation
wheel.  The hdc_rs_py calls are intentionally visible in this script:

    Python -> hdc_rs_py -> Rust blocking::HdcClient -> Rust async HdcClient

The script is opt-in and requires HDC_TEST_DEVICE_ID.  It creates unique
mapping task strings, records every command and response under
target/revalidation-20260904/tls-forward, and attempts exact cleanup in a
finally block.  The ``openssl`` backend additionally creates unique remote
paths for temporary PEM files and process output.  It does not change app
state, device mode, permissions, or reboot state.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import socket
import ssl
import subprocess
import sys
import threading
import time
import uuid
from pathlib import Path
from typing import Any, Callable, Optional

try:
    from hdc_rs_py import HdcClient
except ImportError as error:  # pragma: no cover - depends on the validation venv
    HdcClient = None  # type: ignore[assignment,misc]
    _HDC_IMPORT_ERROR = error
else:
    _HDC_IMPORT_ERROR = None


SERVER_ADDR = os.environ.get("HDC_TEST_SERVER_ADDR", "127.0.0.1:8710")
DEVICE_ID = os.environ.get("HDC_TEST_DEVICE_ID", "")
FORWARD_BACKEND = os.environ.get("HDC_TEST_FORWARD_BACKEND", "bridge").strip().lower()
REMOTE_DIR = os.environ.get(
    "HDC_TEST_REMOTE_DIR", "/data/local/tmp/hdc-rs-v020-real"
)
DEVICE_OPENSSL = os.environ.get("HDC_TEST_DEVICE_OPENSSL", "/bin/openssl")


def shell_quote(value: str) -> str:
    """Quote one argument for the device-side POSIX shell."""

    if any(char in value for char in ("\r", "\n", "\x00")):
        raise ValueError("shell argument contains a control character")
    return "'" + value.replace("'", "'\\''") + "'"


def response_ok(label: str, response: str) -> None:
    if response.lstrip().startswith("[Fail]"):
        raise RuntimeError(f"{label} failed: {response}")


def require_device_id() -> str:
    if not DEVICE_ID.strip():
        raise RuntimeError(
            "HDC_TEST_DEVICE_ID is required; refusing to use a personal-device default"
        )
    if any(char in DEVICE_ID for char in ("\r", "\n", "\x00")):
        raise RuntimeError("HDC_TEST_DEVICE_ID contains a control character")
    return DEVICE_ID


def find_host_openssl() -> str:
    configured = os.environ.get("HDC_TEST_HOST_OPENSSL")
    candidates = [configured] if configured else []
    on_windows = os.name == "nt"
    if on_windows:
        candidates.extend(
            [
                r"C:\Program Files\Git\mingw64\bin\openssl.exe",
                r"C:\Program Files\Git\usr\bin\openssl.exe",
                r"C:\Program Files\OpenSSL-Win64\bin\openssl.exe",
            ]
        )
    candidates.append(shutil.which("openssl"))
    for candidate in candidates:
        if candidate and Path(candidate).is_file():
            return str(Path(candidate))
    raise RuntimeError(
        "host openssl was not found; set HDC_TEST_HOST_OPENSSL to an openssl executable"
    )


class RunLog:
    """Append machine-readable command, response, and failure records."""

    def __init__(self, root: Path, run_id: str) -> None:
        self.root = root
        self.run_dir = root / f"run-{run_id}"
        self.run_dir.mkdir(parents=True, exist_ok=True)
        self.path = self.run_dir / "events.jsonl"

    def event(self, kind: str, **fields: Any) -> None:
        record = {
            "utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "kind": kind,
            **fields,
        }
        with self.path.open("a", encoding="utf-8") as stream:
            stream.write(json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n")

    def command(self, label: str, command: Any, **fields: Any) -> None:
        self.event("command", label=label, command=command, **fields)


class DeviceApi:
    """Small wrapper that keeps each terminal task on a fresh client."""

    def __init__(self, address: str, device_id: str, log: RunLog) -> None:
        if HdcClient is None:
            raise RuntimeError(
                f"cannot import hdc_rs_py: {_HDC_IMPORT_ERROR}; install the CPython 3.11 validation wheel"
            )
        self.address = address
        self.device_id = device_id
        self.log = log
        self.log.event(
            "api_chain",
            chain=[
                "Python hdc_rs_py.HdcClient",
                "Rust blocking::HdcClient",
                "Rust async HdcClient",
            ],
        )

    def _client(self, connected: bool) -> Any:
        client = HdcClient(self.address)
        if connected:
            client.connect_device(self.device_id)
        return client

    def shell(self, command: str, label: str) -> str:
        self.log.command(label, command, api="Python -> blocking -> async shell")
        response = self._client(True).shell(command)
        self.log.event("response", label=label, response=response)
        response_ok(label, response)
        return response

    def file_send(self, local: Path, remote: str, label: str) -> str:
        self.log.command(
            label,
            ["file_send", str(local), remote],
            api="Python -> blocking -> async file_send",
        )
        response = self._client(True).file_send(str(local), remote)
        self.log.event("response", label=label, response=response)
        response_ok(label, response)
        return response

    def fport(self, local: int, remote: int, label: str) -> str:
        self.log.command(
            label,
            ["fport", f"tcp:{local}", f"tcp:{remote}"],
            api="Python -> blocking -> async fport",
        )
        response = self._client(True).fport(f"tcp:{local}", f"tcp:{remote}")
        self.log.event("response", label=label, response=response)
        response_ok(label, response)
        return response

    def rport(self, remote: int, local: int, label: str) -> str:
        self.log.command(
            label,
            ["rport", f"tcp:{remote}", f"tcp:{local}"],
            api="Python -> blocking -> async rport",
        )
        response = self._client(True).rport(f"tcp:{remote}", f"tcp:{local}")
        self.log.event("response", label=label, response=response)
        response_ok(label, response)
        return response

    def fport_list(self, label: str) -> list[str]:
        self.log.command(
            label,
            ["fport_list"],
            api="Python -> blocking -> async fport_list",
        )
        response = self._client(False).fport_list()
        self.log.event("response", label=label, response=response)
        return response

    def fport_remove(self, task: str, label: str) -> str:
        self.log.command(
            label,
            ["fport_remove", task],
            api="Python -> blocking -> async fport_remove",
        )
        response = self._client(False).fport_remove(task)
        self.log.event("response", label=label, response=response)
        response_ok(label, response)
        return response


def choose_port_pair(api: DeviceApi, log: RunLog, mode: str) -> tuple[int, int]:
    """Select unique unprivileged ports without removing existing mappings."""

    existing = api.fport_list(f"{mode}.baseline_fport_list")
    used = "\n".join(existing)
    seed = int(time.time() * 1000) ^ os.getpid()
    for offset in range(2000):
        remote = 40000 + ((seed + offset * 7919) % 18000)
        if mode == "fport":
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
                probe.bind(("127.0.0.1", 0))
                local = int(probe.getsockname()[1])
        else:
            local = 40000 + ((seed + offset * 3571) % 18000)
        task = f"tcp:{local} tcp:{remote}"
        if task not in used and f"tcp:{local}" not in used:
            log.event("ports", mode=mode, local=local, remote=remote, task=task)
            return local, remote
    raise RuntimeError(f"unable to choose unused ports for {mode}")


def choose_bridge_device_port(api: DeviceApi, log: RunLog) -> int:
    """Choose a device-side port for the nested fport/rport validation."""

    existing = api.fport_list("bridge.baseline_fport_list")
    used = "\n".join(existing)
    seed = int(time.time() * 1000) ^ os.getpid()
    for offset in range(2000):
        device = 40000 + ((seed + offset * 7919) % 18000)
        if f"tcp:{device}" not in used:
            return device
    raise RuntimeError("unable to choose an unused device-side bridge port")


def make_payload(mode: str, run_id: str) -> bytes:
    header = f"HDC_TLS_{mode.upper()}_HOST_TO_DEVICE_{run_id}|".encode("ascii")
    digest = hashlib.sha256(header).hexdigest().encode("ascii")
    body = (header + digest + b"|" + b"0123456789abcdef" * 512)
    # OpenSSL s_server -rev reverses each newline-terminated line while
    # preserving its final newline. Keep the payload a single known line.
    return body[:4095] + b"\n"


def reverse_line(payload: bytes) -> bytes:
    if payload.endswith(b"\n"):
        return payload[:-1][::-1] + b"\n"
    return payload[::-1]


def recv_exact(stream: socket.socket, size: int, label: str) -> bytes:
    chunks = bytearray()
    while len(chunks) < size:
        piece = stream.recv(size - len(chunks))
        if not piece:
            raise RuntimeError(
                f"{label}: TLS peer closed after {len(chunks)} of {size} bytes"
            )
        chunks.extend(piece)
    return bytes(chunks)


def host_tls_echo_server(
    server_socket: socket.socket,
    cert: Path,
    key: Path,
    payload: bytes,
    log: RunLog,
    result: dict[str, Any],
) -> None:
    """Serve one TLS connection and echo one exact application payload.

    The listener runs on the host so this validation remains usable on device
    images whose shell SELinux domain cannot bind a TCP socket.  The connection
    still traverses both the device-side reverse listener and the forward
    listener before reaching this endpoint.
    """

    try:
        raw, peer = server_socket.accept()
        log.event("tcp_accept", mode="host_bridge", peer=str(peer))
        with raw:
            context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
            context.minimum_version = ssl.TLSVersion.TLSv1_2
            context.load_cert_chain(certfile=str(cert), keyfile=str(key))
            with context.wrap_socket(raw, server_side=True) as tls:
                tls.settimeout(15.0)
                result["version"] = tls.version()
                result["cipher"] = tls.cipher()
                log.event(
                    "tls_handshake",
                    mode="host_bridge",
                    version=tls.version(),
                    cipher=tls.cipher(),
                )
                received = recv_exact(tls, len(payload), "host bridge application payload")
                result["received"] = received
                if received != payload:
                    raise RuntimeError(
                        "host bridge server received an unexpected payload: "
                        f"expected_sha256={hashlib.sha256(payload).hexdigest()}, "
                        f"got_sha256={hashlib.sha256(received).hexdigest()}"
                    )
                tls.sendall(received)
                result["echoed"] = received
    except BaseException as error:
        result["error"] = error
        log.event("host_bridge_server_failure", error=str(error))


def connect_retry(host: str, port: int, timeout: float, log: RunLog) -> socket.socket:
    deadline = time.monotonic() + timeout
    last_error: Optional[BaseException] = None
    while time.monotonic() < deadline:
        try:
            sock = socket.create_connection((host, port), timeout=min(2.0, timeout))
            sock.settimeout(timeout)
            log.event("tcp_connect", host=host, port=port)
            return sock
        except OSError as error:
            last_error = error
            time.sleep(0.15)
    raise RuntimeError(f"timed out connecting to {host}:{port}: {last_error}")


def create_certificate(host_openssl: str, run_dir: Path, log: RunLog) -> tuple[Path, Path]:
    cert = run_dir / "peer-cert.pem"
    key = run_dir / "peer-key.pem"
    command = [
        host_openssl,
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-sha256",
        "-days",
        "2",
        "-keyout",
        str(key),
        "-out",
        str(cert),
        "-subj",
        "/CN=hdc-rs-tls-forward",
        "-addext",
        "subjectAltName=DNS:localhost,IP:127.0.0.1",
    ]
    log.command("host.generate_certificate", command)
    completed = subprocess.run(
        command,
        capture_output=True,
        text=True,
        check=False,
    )
    log.event(
        "response",
        label="host.generate_certificate",
        returncode=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
    )
    if completed.returncode != 0:
        raise RuntimeError(f"certificate generation failed: {completed.stderr}")
    if not cert.is_file() or not key.is_file():
        raise RuntimeError("certificate generation did not create cert/key")
    log.event(
        "certificate",
        cert=str(cert),
        key=str(key),
        sha256=hashlib.sha256(cert.read_bytes()).hexdigest(),
    )
    return cert, key


def remote_paths(remote_dir: str, mode: str, run_id: str) -> dict[str, str]:
    prefix = f"{remote_dir.rstrip('/')}/hdc_tls_{mode}_{run_id}"
    return {
        "cert": f"{prefix}.crt",
        "key": f"{prefix}.key",
        "output": f"{prefix}.out",
        "pid": f"{prefix}.pid",
        "error": f"{prefix}.err",
    }


def start_server_command(
    paths: dict[str, str],
    port: int,
    token: bytes,
) -> str:
    token_text = token.decode("ascii")
    return (
        f"rm -f {shell_quote(paths['output'])} {shell_quote(paths['pid'])} "
        f"{shell_quote(paths['error'])}; "
        f"printf '%s' {shell_quote(token_text)} > {shell_quote(paths['output'])} || exit 1; "
        f"(sleep 20) | {shell_quote(DEVICE_OPENSSL)} s_server -4 -accept {port} "
        f"-cert {shell_quote(paths['cert'])} -key {shell_quote(paths['key'])} "
        f"-quiet -rev -naccept 1 >> {shell_quote(paths['output'])} "
        f"2> {shell_quote(paths['error'])} & echo $! > {shell_quote(paths['pid'])}"
    )


def start_client_command(
    paths: dict[str, str],
    port: int,
    token: bytes,
) -> str:
    token_text = token.decode("ascii")
    return (
        f"rm -f {shell_quote(paths['output'])} {shell_quote(paths['pid'])} "
        f"{shell_quote(paths['error'])}; "
        f"(printf '%s' {shell_quote(token_text)}; sleep 20) "
        f"| {shell_quote(DEVICE_OPENSSL)} s_client -4 -connect 127.0.0.1:{port} "
        f"-CAfile {shell_quote(paths['cert'])} -verify 1 -verify_return_error "
        f"-verify_hostname localhost -quiet -nocommands -ign_eof "
        f"> {shell_quote(paths['output'])} 2> {shell_quote(paths['error'])} & "
        f"echo $! > {shell_quote(paths['pid'])}"
    )


def cleanup_command(paths: dict[str, str]) -> str:
    # Kill only the PID written by this run, then remove only its exact paths.
    return (
        f"if [ -f {shell_quote(paths['pid'])} ]; then "
        f"kill \"$(cat {shell_quote(paths['pid'])})\" 2>/dev/null || true; fi; "
        f"rm -f {shell_quote(paths['output'])} {shell_quote(paths['pid'])} "
        f"{shell_quote(paths['error'])} {shell_quote(paths['cert'])} "
        f"{shell_quote(paths['key'])}"
    )


def remote_error(api: DeviceApi, paths: dict[str, str], label: str) -> str:
    try:
        response = api.shell(
            f"if [ -f {shell_quote(paths['error'])} ]; then cat {shell_quote(paths['error'])}; fi",
            label,
        )
        return response
    except Exception as error:  # best effort diagnostics during failure handling
        return f"<unable to read device error log: {error}>"


def wait_remote_contains(
    api: DeviceApi,
    path: str,
    expected: bytes,
    label: str,
    log: RunLog,
    timeout: float = 8.0,
) -> str:
    deadline = time.monotonic() + timeout
    last_response = ""
    while time.monotonic() < deadline:
        try:
            last_response = api.shell(
                f"if [ -f {shell_quote(path)} ]; then cat {shell_quote(path)}; fi",
                label,
            )
            if expected in last_response.encode("utf-8", errors="surrogateescape"):
                log.event(
                    "remote_output_contains",
                    label=label,
                    path=path,
                    expected_len=len(expected),
                )
                return last_response
        except Exception as error:
            last_response = f"<read failed: {error}>"
        time.sleep(0.15)
    raise RuntimeError(
        f"timed out waiting for {label} in {path}; expected_len={len(expected)}, "
        f"last_response={last_response!r}"
    )


def verify_mapping(api: DeviceApi, mode: str, local: int, remote: int) -> None:
    task = f"tcp:{local} tcp:{remote}"
    entries = api.fport_list(f"{mode}.verify_fport_list")
    matching = [entry for entry in entries if f"tcp:{local}" in entry and f"tcp:{remote}" in entry]
    if not matching:
        raise RuntimeError(f"{mode} mapping not listed: task={task!r}, entries={entries!r}")
    if mode == "rport" and not any("[Reverse]" in entry for entry in matching):
        raise RuntimeError(f"rport mapping lacks [Reverse] marker: {matching!r}")


def remove_mapping(api: DeviceApi, task: str, log: RunLog) -> Optional[str]:
    try:
        response = api.fport_remove(task, "cleanup.fport_remove")
        remaining = api.fport_list("cleanup.fport_list")
        if any(task.split()[0] in entry and task.split()[1] in entry for entry in remaining):
            raise RuntimeError(f"mapping remains after cleanup: {task}; {remaining!r}")
        return response
    except Exception as error:
        log.event("cleanup_error", step="fport_remove", error=str(error), task=task)
        return None


def cleanup_remote(api: DeviceApi, paths: dict[str, str], log: RunLog) -> bool:
    try:
        api.shell(cleanup_command(paths), "cleanup.device_process_and_files")
        return True
    except Exception as error:
        log.event("cleanup_error", step="device_process_and_files", error=str(error))
        return False


def run_fport(
    api: DeviceApi,
    cert: Path,
    key: Path,
    log: RunLog,
    run_id: str,
) -> None:
    mode = "fport"
    local, remote = choose_port_pair(api, log, mode)
    task = f"tcp:{local} tcp:{remote}"
    paths = remote_paths(REMOTE_DIR, mode, run_id)
    device_token = f"HDC_TLS_FPORT_DEVICE_TO_HOST_{run_id}".encode("ascii")
    payload = make_payload(mode, run_id)
    mapping_attempted = False
    cleanup_errors: list[str] = []
    body_error: Optional[BaseException] = None
    try:
        api.shell(f"mkdir -p {shell_quote(REMOTE_DIR)}", "fport.mkdir_remote_dir")
        api.file_send(cert, paths["cert"], "fport.upload_certificate")
        api.file_send(key, paths["key"], "fport.upload_private_key")
        service = start_server_command(paths, remote, device_token)
        log.command("fport.start_device_openssl_s_server", service)
        api.shell(service, "fport.start_device_openssl_s_server")

        mapping_attempted = True
        api.fport(local, remote, "fport.create")
        verify_mapping(api, mode, local, remote)

        context = ssl.create_default_context(ssl.Purpose.SERVER_AUTH, cafile=str(cert))
        context.minimum_version = ssl.TLSVersion.TLSv1_2
        context.check_hostname = True
        context.verify_mode = ssl.CERT_REQUIRED
        with connect_retry("127.0.0.1", local, 15.0, log) as raw:
            with context.wrap_socket(raw, server_hostname="localhost") as tls:
                tls.settimeout(15.0)
                challenge = device_token + b"|" + payload
                expected_reply = reverse_line(challenge)
                log.event(
                    "tls_handshake",
                    mode=mode,
                    version=tls.version(),
                    cipher=tls.cipher(),
                )
                tls.sendall(challenge)
                echoed = recv_exact(
                    tls, len(expected_reply), "fport complete -rev application reply"
                )
                if echoed != expected_reply:
                    raise RuntimeError(
                        f"fport -rev reply mismatch: expected_sha256={hashlib.sha256(expected_reply).hexdigest()}, "
                        f"got_sha256={hashlib.sha256(echoed).hexdigest()}"
                    )
                output = wait_remote_contains(
                    api,
                    paths["output"],
                    expected_reply,
                    "fport.verify_device_output",
                    log,
                )
                if device_token not in output.encode("utf-8", errors="surrogateescape"):
                    raise RuntimeError("fport device output did not retain the device token")
                log.event(
                    "application_payload_pass",
                    mode=mode,
                    device_token=device_token.decode("ascii"),
                    host_payload_len=len(challenge),
                    host_payload_sha256=hashlib.sha256(payload).hexdigest(),
                    echoed_len=len(echoed),
                    echoed_sha256=hashlib.sha256(echoed).hexdigest(),
                )
    except BaseException as error:
        body_error = error
        log.event("failure_attempt", mode=mode, error=str(error), device_error=remote_error(api, paths, "fport.read_device_error"))
    finally:
        if mapping_attempted:
            if remove_mapping(api, task, log) is None:
                cleanup_errors.append("fport mapping cleanup failed")
        if not cleanup_remote(api, paths, log):
            cleanup_errors.append("device process/file cleanup failed")
    if cleanup_errors and body_error is None:
        body_error = RuntimeError("; ".join(cleanup_errors))
    elif cleanup_errors:
        log.event("cleanup_error", mode=mode, errors=cleanup_errors)
    if body_error is not None:
        raise RuntimeError(f"{mode} TLS data-plane failed: {body_error}") from body_error


def run_rport(
    api: DeviceApi,
    cert: Path,
    key: Path,
    log: RunLog,
    run_id: str,
) -> None:
    mode = "rport"
    local, remote = choose_port_pair(api, log, mode)
    task = f"tcp:{remote} tcp:{local}"
    paths = remote_paths(REMOTE_DIR, mode, run_id)
    device_token = f"HDC_TLS_RPORT_DEVICE_TO_HOST_{run_id}".encode("ascii")
    payload = make_payload(mode, run_id)
    mapping_attempted = False
    server_socket: Optional[socket.socket] = None
    body_error: Optional[BaseException] = None
    try:
        server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server_socket.bind(("127.0.0.1", local))
        server_socket.listen(1)
        server_socket.settimeout(15.0)
        log.event("host_tls_server_listen", mode=mode, host="127.0.0.1", port=local)

        api.shell(f"mkdir -p {shell_quote(REMOTE_DIR)}", "rport.mkdir_remote_dir")
        api.file_send(cert, paths["cert"], "rport.upload_certificate")
        api.file_send(key, paths["key"], "rport.upload_private_key")
        mapping_attempted = True
        api.rport(remote, local, "rport.create")
        verify_mapping(api, mode, local, remote)

        service = start_client_command(paths, remote, device_token)
        log.command("rport.start_device_openssl_s_client", service)
        api.shell(service, "rport.start_device_openssl_s_client")

        raw, peer = server_socket.accept()
        log.event("tcp_accept", mode=mode, peer=str(peer))
        with raw:
            context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
            context.minimum_version = ssl.TLSVersion.TLSv1_2
            context.load_cert_chain(certfile=str(cert), keyfile=str(key))
            with context.wrap_socket(raw, server_side=True) as tls:
                tls.settimeout(15.0)
                received = recv_exact(tls, len(device_token), "rport device token")
                if received != device_token:
                    raise RuntimeError(
                        f"rport device token mismatch: expected {device_token!r}, got {received!r}"
                    )
                log.event(
                    "tls_handshake_and_device_payload",
                    mode=mode,
                    version=tls.version(),
                    cipher=tls.cipher(),
                    device_token=received.decode("ascii"),
                )
                tls.sendall(payload)
            output = wait_remote_contains(
                api,
                paths["output"],
                payload,
                "rport.verify_device_output",
                log,
            )
            if payload not in output.encode("utf-8", errors="surrogateescape"):
                raise RuntimeError("rport device output did not contain the complete host payload")
            log.event(
                "application_payload_pass",
                mode=mode,
                device_token=received.decode("ascii"),
                host_payload_len=len(payload),
                host_payload_sha256=hashlib.sha256(payload).hexdigest(),
                echoed_len=len(payload),
                echoed_sha256=hashlib.sha256(payload).hexdigest(),
            )
    except BaseException as error:
        body_error = error
        log.event("failure_attempt", mode=mode, error=str(error), device_error=remote_error(api, paths, "rport.read_device_error"))
    finally:
        if server_socket is not None:
            server_socket.close()
        cleanup_errors: list[str] = []
        if mapping_attempted and remove_mapping(api, task, log) is None:
            cleanup_errors.append("rport mapping cleanup failed")
        if not cleanup_remote(api, paths, log):
            cleanup_errors.append("device process/file cleanup failed")
        if cleanup_errors and body_error is None:
            body_error = RuntimeError("; ".join(cleanup_errors))
        elif cleanup_errors:
            log.event("cleanup_error", mode=mode, errors=cleanup_errors)
    if body_error is not None:
        raise RuntimeError(f"{mode} TLS data-plane failed: {body_error}") from body_error


def run_host_bridge(
    api: DeviceApi,
    cert: Path,
    key: Path,
    log: RunLog,
    run_id: str,
    requested_mode: str,
) -> None:
    """Verify both directions through a nested host TLS echo connection.

    The host client connects to A, fport carries A -> D, and rport carries
    D -> B where the host TLS echo server listens.  The exact reply therefore
    exercises the complete A/fport/D/rport/B path in both directions without
    requiring a process in the device shell domain to bind a TCP socket.
    """

    device_port = choose_bridge_device_port(api, log)
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.bind(("127.0.0.1", 0))
        local_port = int(probe.getsockname()[1])
    payload = make_payload("fport_rport", run_id)
    fport_task = f"tcp:{local_port} tcp:{device_port}"

    server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server_socket.bind(("127.0.0.1", 0))
    server_socket.listen(1)
    server_socket.settimeout(15.0)
    host_port = int(server_socket.getsockname()[1])
    rport_task = f"tcp:{device_port} tcp:{host_port}"
    log.event(
        "ports",
        mode="host_bridge",
        requested_mode=requested_mode,
        local=local_port,
        device=device_port,
        host=host_port,
        fport_task=fport_task,
        rport_task=rport_task,
    )
    log.event(
        "host_tls_server_listen",
        mode="host_bridge",
        host="127.0.0.1",
        port=host_port,
    )

    server_result: dict[str, Any] = {}
    server_thread = threading.Thread(
        target=host_tls_echo_server,
        args=(server_socket, cert, key, payload, log, server_result),
        name="hdc-rs-tls-echo",
        daemon=True,
    )
    fport_attempted = False
    rport_attempted = False
    cleanup_errors: list[str] = []
    body_error: Optional[BaseException] = None
    try:
        server_thread.start()

        rport_attempted = True
        api.rport(device_port, host_port, "bridge.rport.create")
        verify_mapping(api, "rport", device_port, host_port)

        fport_attempted = True
        api.fport(local_port, device_port, "bridge.fport.create")
        verify_mapping(api, "fport", local_port, device_port)

        context = ssl.create_default_context(ssl.Purpose.SERVER_AUTH, cafile=str(cert))
        context.minimum_version = ssl.TLSVersion.TLSv1_2
        context.check_hostname = True
        context.verify_mode = ssl.CERT_REQUIRED
        with connect_retry("127.0.0.1", local_port, 15.0, log) as raw:
            with context.wrap_socket(raw, server_hostname="localhost") as tls:
                tls.settimeout(15.0)
                log.event(
                    "tls_handshake",
                    mode="host_bridge_client",
                    version=tls.version(),
                    cipher=tls.cipher(),
                )
                tls.sendall(payload)
                echoed = recv_exact(tls, len(payload), "host bridge complete echo")
                if echoed != payload:
                    raise RuntimeError(
                        "host bridge reply mismatch: "
                        f"expected_sha256={hashlib.sha256(payload).hexdigest()}, "
                        f"got_sha256={hashlib.sha256(echoed).hexdigest()}"
                    )

        server_thread.join(timeout=20.0)
        if server_thread.is_alive():
            raise RuntimeError("host bridge TLS echo server did not finish")
        if "error" in server_result:
            raise RuntimeError(f"host bridge TLS echo server failed: {server_result['error']}")
        received = server_result.get("received")
        if received != payload:
            raise RuntimeError(
                "host bridge server did not receive the complete payload: "
                f"expected_sha256={hashlib.sha256(payload).hexdigest()}, "
                f"got_sha256={hashlib.sha256(received or b'').hexdigest()}"
            )
        payload_fields = {
            "transport": "fport A -> device D -> rport B",
            "host_payload_len": len(payload),
            "host_payload_sha256": hashlib.sha256(payload).hexdigest(),
            "echoed_len": len(echoed),
            "echoed_sha256": hashlib.sha256(echoed).hexdigest(),
            "fport_task": fport_task,
            "rport_task": rport_task,
        }
        for mode in ("fport", "rport"):
            log.event("application_payload_pass", mode=mode, **payload_fields)
    except BaseException as error:
        body_error = error
        log.event(
            "failure_attempt",
            mode="host_bridge",
            requested_mode=requested_mode,
            error=str(error),
            server_result={
                key: str(value) if isinstance(value, BaseException) else value
                for key, value in server_result.items()
                if key != "received" and key != "echoed"
            },
        )
    finally:
        try:
            server_socket.close()
        except OSError as error:
            cleanup_errors.append(f"host server socket close failed: {error}")
        if server_thread.is_alive():
            server_thread.join(timeout=2.0)
            if server_thread.is_alive():
                cleanup_errors.append("host TLS echo server thread did not stop")
        if fport_attempted and remove_mapping(api, fport_task, log) is None:
            cleanup_errors.append("fport mapping cleanup failed")
        if rport_attempted and remove_mapping(api, rport_task, log) is None:
            cleanup_errors.append("rport mapping cleanup failed")
    if cleanup_errors and body_error is None:
        body_error = RuntimeError("; ".join(cleanup_errors))
    elif cleanup_errors:
        log.event("cleanup_error", mode="host_bridge", errors=cleanup_errors)
    if body_error is not None:
        raise RuntimeError(f"host bridge TLS data-plane failed: {body_error}") from body_error


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--mode",
        choices=("fport", "rport", "all"),
        default="all",
        help="data-plane direction to verify (default: all)",
    )
    parser.add_argument(
        "--backend",
        choices=("bridge", "openssl"),
        default=None,
        help="validation backend (default: HDC_TEST_FORWARD_BACKEND or bridge)",
    )
    parser.add_argument(
        "--log-dir",
        type=Path,
        default=None,
        help="persistent event-log root (default: target/revalidation-20260904/tls-forward)",
    )
    parser.add_argument(
        "--plan",
        action="store_true",
        help="print the planned operations without importing the wheel or touching the device",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    backend = args.backend or FORWARD_BACKEND
    if backend not in ("bridge", "openssl"):
        raise RuntimeError(
            "HDC_TEST_FORWARD_BACKEND must be either 'bridge' or 'openssl'"
        )
    if backend == "bridge" and args.mode != "all":
        raise RuntimeError(
            "the bridge backend validates fport and rport together; use --mode all"
        )
    if args.plan:
        device_operations = (
            [
                "start a host TLS echo server on B",
                "create rport tcp:D tcp:B",
                "create fport tcp:A tcp:D",
                "connect host TLS client to A and verify exact echo",
                "remove both mappings and confirm fport ls is empty",
            ]
            if backend == "bridge"
            else [
                "upload temporary PEM cert/key",
                "openssl s_server -rev or s_client",
                "capture output in a per-run regular file",
                "exact PID/path cleanup",
            ]
        )
        print(
            json.dumps(
                {
                    "mode": args.mode,
                    "backend": backend,
                    "server": SERVER_ADDR,
                    "device_id": DEVICE_ID or "<HDC_TEST_DEVICE_ID required>",
                    "device_openssl": DEVICE_OPENSSL,
                    "device_operations": device_operations,
                    "api_chain": "Python -> Rust blocking -> Rust async",
                    "success_condition": "TLS application payload reaches host echo and returns exactly",
                },
                indent=2,
            )
        )
        return 0

    device_id = require_device_id()
    if not REMOTE_DIR.startswith("/"):
        raise RuntimeError("HDC_TEST_REMOTE_DIR must be an absolute device path")
    if any(char in DEVICE_OPENSSL for char in ("\r", "\n", "\x00")):
        raise RuntimeError("HDC_TEST_DEVICE_OPENSSL contains a control character")
    host_openssl = find_host_openssl()

    script_path = Path(__file__).resolve()
    repo_root = script_path.parents[2]
    log_root = args.log_dir or repo_root / "target" / "revalidation-20260904" / "tls-forward"
    log_root.mkdir(parents=True, exist_ok=True)
    run_id = time.strftime("%Y%m%d%H%M%S", time.gmtime()) + "-" + uuid.uuid4().hex[:10]
    log = RunLog(log_root, run_id)
    log.event(
        "run_start",
        mode=args.mode,
        backend=backend,
        server=SERVER_ADDR,
        device_id=device_id,
        remote_dir=REMOTE_DIR,
        device_openssl=DEVICE_OPENSSL,
        host_openssl=host_openssl,
        script=str(script_path),
    )

    cert: Optional[Path] = None
    key: Optional[Path] = None
    failures: list[str] = []
    try:
        cert, key = create_certificate(host_openssl, log.run_dir, log)
        api = DeviceApi(SERVER_ADDR, device_id, log)
        if backend == "bridge":
            try:
                run_host_bridge(api, cert, key, log, run_id, args.mode)
                if args.mode in ("fport", "all"):
                    print("fport TLS application payload roundtrip: PASS")
                if args.mode in ("rport", "all"):
                    print("rport TLS application payload roundtrip: PASS")
            except Exception as error:
                failures.append(str(error))
                if args.mode in ("fport", "all"):
                    print(
                        f"fport TLS application payload roundtrip: FAIL ({error})",
                        file=sys.stderr,
                    )
                if args.mode in ("rport", "all"):
                    print(
                        f"rport TLS application payload roundtrip: FAIL ({error})",
                        file=sys.stderr,
                    )
        else:
            if args.mode in ("fport", "all"):
                try:
                    run_fport(api, cert, key, log, run_id)
                    print("fport TLS application payload roundtrip: PASS")
                except Exception as error:
                    failures.append(str(error))
                    print(
                        f"fport TLS application payload roundtrip: FAIL ({error})",
                        file=sys.stderr,
                    )
            if args.mode in ("rport", "all"):
                try:
                    run_rport(api, cert, key, log, run_id)
                    print("rport TLS application payload roundtrip: PASS")
                except Exception as error:
                    failures.append(str(error))
                    print(
                        f"rport TLS application payload roundtrip: FAIL ({error})",
                        file=sys.stderr,
                    )
    finally:
        # Certificates/private keys are temporary validation material. Keep the
        # event log, but remove both PEM files even on a failed attempt.
        temporary_files = [path for path in (cert, key) if path is not None]
        temporary_files.extend(log.run_dir.glob("peer-*.pem"))
        seen: set[Path] = set()
        for temporary in temporary_files:
            if temporary in seen:
                continue
            seen.add(temporary)
            if temporary is not None:
                try:
                    temporary.unlink()
                    log.event("local_cleanup", path=str(temporary))
                except FileNotFoundError:
                    pass
        log.event("run_end", failures=failures)

    if failures:
        print(f"TLS data-plane validation failed; event log: {log.path}", file=sys.stderr)
        return 1
    print(f"TLS data-plane validation passed; event log: {log.path}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"FAILED: {error}", file=sys.stderr)
        raise SystemExit(1)
