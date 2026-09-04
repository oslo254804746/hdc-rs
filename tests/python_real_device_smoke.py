"""
Python real-device smoke test for hdc-rs (hdc_rs_py).

This script is opt-in through HDC_TEST_DEVICE_ID. It creates a fresh client
for each one-shot terminal task and cleans temporary host files, the remote
test file, and any port mappings in a finally block.
"""

import hashlib
import os
import shutil
import sys
import tempfile
import time

try:
    import hdc_rs_py
    from hdc_rs_py import HdcClient
except ImportError as err:
    print(f"FAILED to import hdc_rs_py: {err}", file=sys.stderr)
    sys.exit(1)


def required_env(name):
    value = os.environ.get(name)
    if not value or not value.strip():
        raise RuntimeError(
            f"{name} is required for the real-device smoke test; refusing a personal-device default"
        )
    return value


SERVER_ADDR = os.environ.get("HDC_TEST_SERVER_ADDR", "127.0.0.1:8710")
DEVICE_ID = required_env("HDC_TEST_DEVICE_ID")
REMOTE_DIR = os.environ.get(
    "HDC_TEST_REMOTE_DIR", "/data/local/tmp/hdc-rs-v020-real"
)
SHELL_STATUS_MARKER = "HDC_PY_SHELL_RC:"


def shell_quote(value):
    """Quote one dynamic argument for the device-side POSIX shell."""
    if any(char in value for char in ("\r", "\n", "\x00")):
        raise ValueError("shell argument contains a control character")
    return "'" + value.replace("'", "'\\''") + "'"


def assert_command_ok(label, response):
    if response.lstrip().startswith("[Fail]"):
        raise AssertionError(f"{label} failed: {response}")


def shell_command_with_status(command):
    """Append an unambiguous exit-status marker to a device shell command."""
    return (
        f'{command}; rc=$?; printf \'\\n{SHELL_STATUS_MARKER}%s\\n\' "$rc"'
    )


def shell_output_and_status(response, label):
    """Separate command output from the explicit device-shell exit status."""
    output, status_text = response.rsplit(SHELL_STATUS_MARKER, 1)
    try:
        status = int(status_text.strip())
    except ValueError as error:
        raise RuntimeError(
            f"{label} returned an invalid shell status marker: {response!r}"
        ) from error
    return output, status


def checked_shell(client, command, label):
    """Run a shell command and reject nonzero status or HDC failure text."""
    response = client.shell(shell_command_with_status(command))
    output, status = shell_output_and_status(response, label)
    if status != 0:
        raise RuntimeError(f"{label} failed (status {status}): {output}")
    assert_command_ok(label, output)
    return output


def new_client():
    return HdcClient(SERVER_ADDR)


def new_connected_client():
    client = new_client()
    client.connect_device(DEVICE_ID)
    return client


def cleanup_remote_file(remote_file, errors):
    if remote_file is None:
        return
    try:
        client = new_connected_client()
        checked_shell(
            client,
            f"rm -f {shell_quote(remote_file)}",
            "remote smoke-file cleanup",
        )
    except Exception as exc:  # best effort, reported after the smoke body
        errors.append(f"remote file cleanup failed: {exc}")


def cleanup_forward_tasks(forward_tasks, errors):
    for task in forward_tasks:
        try:
            response = new_client().fport_remove(task)
            assert_command_ok(f"forward cleanup ({task})", response)
        except Exception as exc:  # best effort, reported after the smoke body
            errors.append(f"forward cleanup failed for {task}: {exc}")


def run_smoke():
    print(f"=== Starting Python real device smoke test on device: {DEVICE_ID} ===")
    remote_file = None
    forward_tasks = []
    test_error = None
    cleanup_errors = []
    temp_workspace = tempfile.mkdtemp(prefix="hdc_py_smoke_")

    try:
        # 1. Server one-shots
        print("[1/7] Testing server one-shots: version, list_targets, check_server...")
        c_ver = new_client()
        ver = c_ver.version()
        assert "Ver:" in ver, f"Unexpected version: {ver}"
        print(f"  Version: {ver.strip()}")

        c_list = new_client()
        targets = c_list.list_targets()
        assert DEVICE_ID in targets, f"Device {DEVICE_ID} not found in targets: {targets}"
        print(f"  Found targets: {targets}")

        c_srv = new_client()
        srv = c_srv.check_server()
        assert srv, "check_server returned empty string"
        print(f"  check_server: {srv.strip()}")

        # 2. Connect + shell
        print("[2/7] Testing connect + consecutive shell calls...")
        c_sh = new_connected_client()
        out1 = checked_shell(c_sh, "echo PY_SHELL_TOKEN_1", "shell 1")
        assert "PY_SHELL_TOKEN_1" in out1, f"shell 1 failed: {out1}"
        out2 = checked_shell(c_sh, "echo PY_SHELL_TOKEN_2", "shell 2")
        assert "PY_SHELL_TOKEN_2" in out2, f"shell 2 failed: {out2}"
        print("  Consecutive shell calls succeeded")

        # 3. Small file roundtrip
        print("[3/7] Testing file send/recv roundtrip with SHA-256 verification...")
        c_setup = new_connected_client()
        checked_shell(
            c_setup,
            f"mkdir -p {shell_quote(REMOTE_DIR)}",
            "remote test directory setup",
        )

        run_id = int(time.time() * 1000)
        remote_file = f"{REMOTE_DIR}/py_smoke_{run_id}.txt"
        local_src = os.path.join(temp_workspace, "source.txt")
        local_dst = os.path.join(temp_workspace, "destination.txt")

        test_content = b"PYTHON_SMOKE_TEST_PAYLOAD_" + str(run_id).encode() + b"\n"
        with open(local_src, "wb") as file_handle:
            file_handle.write(test_content)
        src_hash = hashlib.sha256(test_content).hexdigest()

        c_send = new_connected_client()
        send_res = c_send.file_send(local_src, remote_file)
        assert_command_ok("file_send", send_res)

        c_recv = new_connected_client()
        recv_res = c_recv.file_recv(remote_file, local_dst)
        assert_command_ok("file_recv", recv_res)

        with open(local_dst, "rb") as file_handle:
            dst_content = file_handle.read()
        dst_hash = hashlib.sha256(dst_content).hexdigest()
        assert src_hash == dst_hash, f"SHA-256 mismatch: {src_hash} != {dst_hash}"
        assert dst_content == test_content, "file roundtrip payload differs despite matching hash"
        print(f"  File roundtrip SHA-256 verified: {src_hash}")

        # 4. Port forward control plane. Rust's ignored acceptance tests cover
        # raw fport/rport payloads; this smoke keeps Python's control checks.
        print("[4/7] Testing fport and rport create/list/remove...")
        base_port = 30000 + (run_id % 20000)
        local_port = base_port
        remote_port = base_port + 1
        fport_task = f"tcp:{local_port} tcp:{remote_port}"
        existing_tasks = new_client().fport_list()
        if any(
            f"tcp:{local_port}" in item and f"tcp:{remote_port}" in item
            for item in existing_tasks
        ):
            raise RuntimeError(
                f"prerequisite failed: fport task already exists for {fport_task}; refusing to remove an existing mapping"
            )
        forward_tasks.append(fport_task)

        c_fp = new_connected_client()
        fp_res = c_fp.fport(f"tcp:{local_port}", f"tcp:{remote_port}")
        assert_command_ok("fport", fp_res)

        c_ls = new_client()
        fport_list = c_ls.fport_list()
        assert any(
            f"tcp:{local_port}" in item and f"tcp:{remote_port}" in item
            for item in fport_list
        ), f"Task not in list: {fport_list}"

        c_rm = new_client()
        rm_res = c_rm.fport_remove(fport_task)
        assert_command_ok("fport_remove", rm_res)
        remaining_tasks = new_client().fport_list()
        assert not any(
            f"tcp:{local_port}" in item and f"tcp:{remote_port}" in item
            for item in remaining_tasks
        ), f"fport task still listed after removal: {remaining_tasks}"
        forward_tasks.remove(fport_task)

        r_remote = base_port + 2
        r_local = base_port + 3
        rport_task = f"tcp:{r_remote} tcp:{r_local}"
        existing_tasks = new_client().fport_list()
        if any(
            f"tcp:{r_remote}" in item and f"tcp:{r_local}" in item
            for item in existing_tasks
        ):
            raise RuntimeError(
                f"prerequisite failed: rport task already exists for {rport_task}; refusing to remove an existing mapping"
            )
        forward_tasks.append(rport_task)

        c_rp = new_connected_client()
        rp_res = c_rp.rport(f"tcp:{r_remote}", f"tcp:{r_local}")
        assert_command_ok("rport", rp_res)

        c_ls2 = new_client()
        fport_list2 = c_ls2.fport_list()
        assert any(
            f"tcp:{r_remote}" in item
            and f"tcp:{r_local}" in item
            and "[Reverse]" in item
            for item in fport_list2
        ), f"Reverse task not in list: {fport_list2}"

        c_rm2 = new_client()
        rm_res2 = c_rm2.fport_remove(rport_task)
        assert_command_ok("rport remove", rm_res2)
        remaining_tasks = new_client().fport_list()
        assert not any(
            f"tcp:{r_remote}" in item and f"tcp:{r_local}" in item
            for item in remaining_tasks
        ), f"rport task still listed after removal: {remaining_tasks}"
        forward_tasks.remove(rport_task)
        print("  Forward and reverse port control plane verified")

        # 5. hilog stream callback stop
        print("[5/7] Testing hilog_stream stop on callback...")
        c_hl = new_connected_client()
        hl_called = [0]

        def hilog_cb(line):
            del line
            hl_called[0] += 1
            return False  # stop immediately

        c_hl.hilog_stream(hilog_cb)
        assert hl_called[0] == 1, (
            "hilog callback must be called exactly once when it stops the stream; "
            f"got {hl_called[0]}"
        )
        print("  hilog_stream stopped successfully after one callback")

        # 6. JDWP jpid
        print("[6/7] Testing jpid...")
        c_jpid = new_connected_client()
        pids = c_jpid.jpid()
        assert pids, "jpid returned empty list"
        assert all(pid.strip().isdigit() for pid in pids), (
            f"Non-numeric PID in jpid: {pids}"
        )
        print(f"  jpid returned {len(pids)} PIDs, all numeric")

    except Exception as exc:
        test_error = exc
    finally:
        cleanup_forward_tasks(forward_tasks, cleanup_errors)
        cleanup_remote_file(remote_file, cleanup_errors)
        shutil.rmtree(temp_workspace, ignore_errors=True)

    if cleanup_errors:
        cleanup_message = "; ".join(cleanup_errors)
        if test_error is None:
            raise AssertionError(cleanup_message)
        raise AssertionError(f"{test_error}; {cleanup_message}") from test_error
    if test_error is not None:
        raise test_error
    print("[7/7] Python real device smoke test: ALL PASS!")


if __name__ == "__main__":
    try:
        run_smoke()
    except Exception as exc:
        print(f"FAILED: {exc}", file=sys.stderr)
        sys.exit(1)
