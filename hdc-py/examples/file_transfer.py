"""
文件传输示例
"""

from hdc_rs import HdcClient
import os


def main():
    # 连接到 HDC 服务器和设备
    client = HdcClient("127.0.0.1:8710")
    devices = client.list_targets()
    
    if not devices:
        print("未找到设备")
        return
    
    client.connect_device(devices[0])
    print(f"已连接到设备: {devices[0]}")
    
    # 创建测试文件
    test_file = "test_upload.txt"
    if not os.path.exists(test_file):
        with open(test_file, "w") as f:
            f.write("Hello from Python!\n")
            f.write("This is a test file.\n")
        print(f"创建测试文件: {test_file}")
    
    # 发送文件到设备
    print("\n发送文件到设备...")
    remote_path = "/data/local/tmp/test_from_python.txt"
    result = client.file_send(test_file, remote_path)
    print(f"结果: {result}")
    
    # 从设备接收文件
    print("\n从设备接收文件...")
    local_path = "received_from_device.txt"
    result = client.file_recv(remote_path, local_path)
    print(f"结果: {result}")
    
    # 验证文件内容
    if os.path.exists(local_path):
        print(f"\n接收到的文件内容:")
        with open(local_path, "r") as f:
            print(f.read())
    
    # 使用压缩传输大文件
    print("\n使用压缩传输...")
    result = client.file_send(
        test_file, 
        "/data/local/tmp/compressed.txt",
        compress=True,
        preserve_timestamp=True
    )
    print(f"结果: {result}")


if __name__ == "__main__":
    main()
