"""
基础使用示例 - 列出设备并执行命令
"""

from hdc_rs_py import HdcClient


def main():
    # 连接到 HDC 服务器
    print("连接到 HDC 服务器...")
    client = HdcClient("127.0.0.1:8710")
    
    # 列出所有设备
    print("\n列出所有设备...")
    devices = client.list_targets()
    print(f"找到 {len(devices)} 个设备:")
    for device in devices:
        print(f"  - {device}")
    
    if not devices:
        print("\n未找到设备。请确保设备已连接并且 HDC 服务器正在运行。")
        return
    
    # 连接到第一个设备
    device_id = devices[0]
    print(f"\n连接到设备: {device_id}")
    client.connect_device(device_id)
    
    # 执行一些 shell 命令
    print("\n执行 shell 命令...")
    
    commands = [
        "pwd",
        "ls -l /data/local/tmp",
        "uname -a",
        "date",
    ]
    
    for cmd in commands:
        print(f"\n$ {cmd}")
        try:
            output = client.shell(cmd)
            print(output)
        except Exception as e:
            print(f"错误: {e}")


if __name__ == "__main__":
    main()
