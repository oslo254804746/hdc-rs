#!/usr/bin/env python3
"""
HDC Python Binding - Streaming Features Demo

Demonstrates:
1. hilog_stream: Real-time log streaming with callback
2. monitor_devices: Device list monitoring with callback
"""

import time
from hdc_rs_py import HdcClient

def demo_hilog_stream():
    """Demo: Stream device logs in real-time"""
    print("=== Demo: hilog_stream ===")
    
    client = HdcClient("127.0.0.1:8710")
    
    # Wait for device
    print("Waiting for device...")
    device = client.wait_for_device()
    print(f"Connected to: {device}")
    
    # Connect to specific device
    client.connect_device(device)
    
    # Stream logs with callback
    print("\nStreaming logs (Press Ctrl+C to stop)...")
    log_count = 0
    
    def log_handler(log_chunk):
        nonlocal log_count
        print(log_chunk, end='')  # Print log chunk without extra newline
        log_count += 1
        
        # Optional: Stop after certain count for demo
        # if log_count >= 100:
        #     return False  # Stop streaming
        
        return True  # Continue streaming
    
    try:
        # Stream all logs
        client.hilog_stream(log_handler)
    except KeyboardInterrupt:
        print(f"\nStopped. Received {log_count} log chunks.")
    
    print("\n")


def demo_hilog_stream_filtered():
    """Demo: Stream filtered logs with arguments"""
    print("=== Demo: hilog_stream with filter ===")
    
    client = HdcClient()
    
    # Get first available device
    targets = client.list_targets()
    if not targets:
        print("No device found!")
        return
    
    device = targets[0]
    print(f"Using device: {device}")
    client.connect_device(device)
    
    # Stream logs with tag filter
    print("\nStreaming logs with filter (tag=MyTag, Press Ctrl+C to stop)...")
    
    def log_handler(log_chunk):
        print(log_chunk, end='')
        return True  # Continue streaming
    
    try:
        # Stream logs with specific tag
        client.hilog_stream(log_handler, args="-t MyTag")
    except KeyboardInterrupt:
        print("\nStopped.")
    
    print("\n")


def demo_monitor_devices():
    """Demo: Monitor device connection/disconnection"""
    print("=== Demo: monitor_devices ===")
    
    client = HdcClient()
    
    print("Monitoring device list (Press Ctrl+C to stop)...")
    print("Try connecting/disconnecting devices to see changes.\n")
    
    last_devices = None
    
    def device_monitor(devices):
        nonlocal last_devices
        
        # Print timestamp
        timestamp = time.strftime("%H:%M:%S")
        
        # Detect changes
        if devices != last_devices:
            if last_devices is None:
                # Initial state
                print(f"[{timestamp}] Initial devices: {devices}")
            else:
                # Calculate changes
                added = set(devices) - set(last_devices)
                removed = set(last_devices) - set(devices)
                
                if added:
                    print(f"[{timestamp}] ✓ Device(s) connected: {list(added)}")
                if removed:
                    print(f"[{timestamp}] ✗ Device(s) disconnected: {list(removed)}")
                if not added and not removed:
                    print(f"[{timestamp}] Devices: {devices}")
            
            last_devices = devices[:]  # Copy list
        else:
            # No change (optional: comment out to reduce output)
            # print(f"[{timestamp}] No change: {len(devices)} device(s)")
            pass
        
        return True  # Continue monitoring
    
    try:
        # Monitor with 2 second interval
        client.monitor_devices(device_monitor, interval_secs=2)
    except KeyboardInterrupt:
        print("\nStopped monitoring.")
    
    print("\n")


def demo_monitor_with_auto_action():
    """Demo: Automatic action when device connects"""
    print("=== Demo: monitor_devices with auto-action ===")
    
    client = HdcClient("127.0.0.1:8710")
    
    print("Waiting for device to connect...")
    print("When a device connects, it will automatically run 'getprop ro.product.model'\n")
    
    def device_monitor(devices):
        if len(devices) > 0:
            device = devices[0]
            print(f"Device connected: {device}")
            
            # Auto-connect and run command
            try:
                client.connect_device(device)
                result = client.shell("getprop ro.product.model")
                print(f"Device model: {result.strip()}")
                
                # Stop monitoring after first device
                return False
            except Exception as e:
                print(f"Error: {e}")
                return True  # Continue monitoring
        else:
            print("No devices found, waiting...")
            return True  # Continue monitoring
    
    try:
        # Monitor with 1 second interval for faster response
        client.monitor_devices(device_monitor, interval_secs=1)
    except KeyboardInterrupt:
        print("\nStopped.")
    
    print("\n")


def main():
    """Run all demos"""
    demos = [
        ("1", "Stream all logs", demo_hilog_stream),
        ("2", "Stream filtered logs", demo_hilog_stream_filtered),
        ("3", "Monitor device list", demo_monitor_devices),
        ("4", "Monitor with auto-action", demo_monitor_with_auto_action),
    ]
    
    print("HDC-PY Streaming Features Demo")
    print("=" * 50)
    print("\nAvailable demos:")
    for num, desc, _ in demos:
        print(f"  {num}. {desc}")
    print(f"  0. Run all demos sequentially")
    print()
    
    choice = input("Select demo (0-4): ").strip()
    
    if choice == "0":
        # Run all demos
        for num, desc, func in demos:
            print(f"\n{'='*50}")
            print(f"Running: {desc}")
            print('='*50 + "\n")
            try:
                func()
            except Exception as e:
                print(f"Error: {e}\n")
            
            if num != demos[-1][0]:
                input("Press Enter to continue to next demo...")
    else:
        # Run selected demo
        for num, desc, func in demos:
            if choice == num:
                func()
                return
        
        print("Invalid choice!")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nExiting...")
