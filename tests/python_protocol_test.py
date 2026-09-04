"""Offline regression tests against an installed wheel; no HDC device is used.

Run with the Python interpreter into which the candidate wheel was installed.
The mock server runs in another process because the synchronous binding holds
the GIL while waiting for socket input.
"""

import contextlib
import multiprocessing
import socket
import struct
import unittest


def read_exact(connection, size):
    output = bytearray()
    while len(output) < size:
        chunk = connection.recv(size - len(output))
        if not chunk:
            raise EOFError("Client closed a partial packet")
        output.extend(chunk)
    return bytes(output)


def read_packet(connection):
    size = struct.unpack(">I", read_exact(connection, 4))[0]
    if size > 1024 * 1024:
        raise ValueError("Unexpected mock client packet length")
    return read_exact(connection, size)


def send_packet(connection, data):
    connection.sendall(struct.pack(">I", len(data)) + data)


def serve(pipe, command, chunks):
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        listener.listen(1)
        listener.settimeout(10)
        pipe.send(f"127.0.0.1:{listener.getsockname()[1]}")
        pipe.close()
        connection, _ = listener.accept()
        with connection:
            connection.settimeout(10)
            handshake = b"OHOS HDC" + bytes(4) + struct.pack(">I", 1) + bytes(28)
            send_packet(connection, handshake)
            reply = read_packet(connection)
            assert len(reply) == 44 and reply[:8] == b"OHOS HDC", reply
            received = read_packet(connection)
            assert received == command, (received, command)
            # Send the complete response together so callback-stop cases cannot
            # race a later server write against the client's disconnect.
            connection.sendall(
                b"".join(struct.pack(">I", len(chunk)) + chunk for chunk in chunks)
            )
            connection.shutdown(socket.SHUT_WR)
            assert connection.recv(1) == b"", "Terminal call did not close its channel"


@contextlib.contextmanager
def mock_server(command, chunks):
    context = multiprocessing.get_context("spawn")
    receive, send = context.Pipe(duplex=False)
    process = context.Process(target=serve, args=(send, command, chunks))
    process.start()
    send.close()
    try:
        if not receive.poll(10):
            raise AssertionError("Mock server did not start")
        yield receive.recv()
        process.join(12)
        if process.is_alive():
            raise AssertionError("Mock server did not observe channel closure")
        if process.exitcode != 0:
            raise AssertionError(f"Mock server failed with exit code {process.exitcode}")
    finally:
        receive.close()
        if process.is_alive():
            process.terminate()
        process.join(5)
        process.close()


class WheelProtocolTests(unittest.TestCase):
    def test_hilog_callback_false_is_terminal(self):
        from hdc_rs_py import HdcClient

        with mock_server(b"hilog", [b"first log chunk"]) as address:
            calls = []

            def stop(chunk):
                calls.append(chunk)
                return False

            HdcClient(address).hilog_stream(stop)
            self.assertEqual(calls, ["first log chunk"])

    def test_hilog_stream_preserves_split_unicode_and_lossy_tail(self):
        from hdc_rs_py import HdcClient

        chunks = [b"\xe4", b"\xb8\xad\xe6\x96", b"\x87\n", b"\xfftail", b"\xe6\x96"]
        with mock_server(b"hilog", chunks) as address:
            received = []

            def collect(chunk):
                received.append(chunk)
                return True

            HdcClient(address).hilog_stream(collect)
            self.assertEqual("".join(received), "中文\n\ufffdtail\ufffd")

    def test_buffered_hilog_joins_unicode_before_decoding(self):
        from hdc_rs_py import HdcClient

        with mock_server(b"hilog", [b"\xe4", b"\xb8\xad", b"\xfftail"]) as address:
            self.assertEqual(HdcClient(address).hilog(), "中\ufffdtail")

    def test_uninstall_sends_separate_package_option(self):
        from hdc_rs_py import HdcClient

        with mock_server(
            b"uninstall -n com.example.disposable", [b"Success", b"\x02\x00"]
        ) as address:
            self.assertEqual(
                HdcClient(address).uninstall("com.example.disposable"), "Success"
            )


if __name__ == "__main__":
    unittest.main()
