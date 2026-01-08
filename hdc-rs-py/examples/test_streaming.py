#!/usr/bin/env python3
"""
Quick test for hilog_stream and monitor_devices
"""

import sys
from hdc_rs_py import HdcClient

def test_hilog_stream():
    """Test hilog_stream - print 50 log chunks then stop"""
    print("Testing hilog_stream...")
    
    client = HdcClient("127.0.0.1:8710")
    targets = client.list_targets()
    
    if not targets:
        print("No device found!")
        return False
    
    device = targets[0]
    print(f"Using device: {device}")
    client.connect_device(device)
    
    count = 0
    max_chunks = 50
    
    def log_handler(log_chunk):
        nonlocal count
        count += 1
        print(f"[{count}/{max_chunks}] {log_chunk[:80]}...")  # Print first 80 chars
        
        if count >= max_chunks:
            return False  # Stop after 50 chunks
        return True
    
    try:
        client.hilog_stream(log_handler)
        print(f"\n✓ hilog_stream test passed! Received {count} chunks.")
        return True
    except Exception as e:
        print(f"\n✗ hilog_stream test failed: {e}")
        return False


def test_monitor_devices():
    """Test monitor_devices - monitor for 10 seconds"""
    print("\nTesting monitor_devices...")
    
    client = HdcClient("127.0.0.1:8710")
    
    import time
    start_time = time.time()
    max_duration = 10  # seconds
    poll_count = 0
    
    def device_monitor(devices):
        nonlocal poll_count
        poll_count += 1
        elapsed = time.time() - start_time
        
        print(f"[Poll #{poll_count}] {elapsed:.1f}s - Devices: {devices}")
        
        if elapsed >= max_duration:
            return False  # Stop after 10 seconds
        return True
    
    try:
        client.monitor_devices(device_monitor, interval_secs=1)
        print(f"\n✓ monitor_devices test passed! {poll_count} polls in {max_duration}s.")
        return True
    except Exception as e:
        print(f"\n✗ monitor_devices test failed: {e}")
        return False


def main():
    print("HDC-PY Streaming Features Test")
    print("=" * 50 + "\n")
    
    results = []
    
    # Test 1: hilog_stream
    results.append(("hilog_stream", test_hilog_stream()))
    
    # Test 2: monitor_devices
    results.append(("monitor_devices", test_monitor_devices()))
    
    # Summary
    print("\n" + "=" * 50)
    print("Test Summary:")
    for name, passed in results:
        status = "✓ PASS" if passed else "✗ FAIL"
        print(f"  {status}: {name}")
    
    all_passed = all(r[1] for r in results)
    print("\n" + ("All tests passed!" if all_passed else "Some tests failed!"))
    
    return 0 if all_passed else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\n\nInterrupted by user")
        sys.exit(1)
