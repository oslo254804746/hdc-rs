"""
完整的使用示例 - 展示所有主要功能
"""

from hdc_rs import HdcClient
import os
import time


def print_section(title):
    """打印章节标题"""
    print("\n" + "=" * 60)
    print(f"  {title}")
    print("=" * 60)


def main():
    print_section("HDC-RS Python 完整示例")
    
    # 1. 连接和设备发现
    print_section("1. 连接到 HDC 服务器")
    try:
        client = HdcClient("127.0.0.1:8710")
        print("✓ 成功连接到 HDC 服务器")
    except Exception as e:
        print(f"✗ 连接失败: {e}")
        return
    
    # 2. 列出设备
    print_section("2. 列出所有设备")
    try:
        devices = client.list_targets()
        print(f"找到 {len(devices)} 个设备:")
        for i, device in enumerate(devices, 1):
            print(f"  {i}. {device}")
    except Exception as e:
        print(f"✗ 列出设备失败: {e}")
        return
    
    if not devices:
        print("\n未找到设备。请确保:")
        print("  1. 设备已连接到计算机")
        print("  2. HDC 服务器正在运行")
        print("  3. 设备已启用 USB 调试")
        return
    
    # 3. 连接到设备
    print_section("3. 连接到设备")
    device_id = devices[0]
    try:
        client.connect_device(device_id)
        print(f"✓ 成功连接到设备: {device_id}")
    except Exception as e:
        print(f"✗ 连接设备失败: {e}")
        return
    
    # 4. 执行 Shell 命令
    print_section("4. 执行 Shell 命令")
    commands = [
        ("获取当前目录", "pwd"),
        ("列出文件", "ls -l /data/local/tmp"),
        ("系统信息", "uname -a"),
        ("当前时间", "date"),
    ]
    
    for description, cmd in commands:
        print(f"\n{description}: {cmd}")
        try:
            output = client.shell(cmd)
            print(output.strip())
        except Exception as e:
            print(f"✗ 错误: {e}")
    
    # 5. 文件传输
    print_section("5. 文件传输")
    
    # 创建测试文件
    test_file = "test_comprehensive.txt"
    with open(test_file, "w") as f:
        f.write(f"测试文件\n")
        f.write(f"创建时间: {time.strftime('%Y-%m-%d %H:%M:%S')}\n")
        f.write(f"设备 ID: {device_id}\n")
    
    print(f"创建测试文件: {test_file}")
    
    # 发送文件
    print("\n发送文件到设备...")
    remote_path = "/data/local/tmp/test_comprehensive.txt"
    try:
        result = client.file_send(test_file, remote_path)
        print(f"✓ {result}")
    except Exception as e:
        print(f"✗ 发送失败: {e}")
    
    # 接收文件
    print("\n从设备接收文件...")
    received_file = "received_comprehensive.txt"
    try:
        result = client.file_recv(remote_path, received_file)
        print(f"✓ {result}")
        
        # 验证内容
        if os.path.exists(received_file):
            with open(received_file, "r") as f:
                print("\n接收到的文件内容:")
                print(f.read())
    except Exception as e:
        print(f"✗ 接收失败: {e}")
    
    # 6. 端口转发
    print_section("6. 端口转发")
    
    print("\n设置端口转发 (local:8888 -> device:8888)...")
    try:
        result = client.fport("tcp:8888", "tcp:8888")
        print(f"✓ {result}")
        
        # 移除端口转发
        print("\n移除端口转发...")
        result = client.fport_remove("tcp:8888 tcp:8888")
        print(f"✓ {result}")
    except Exception as e:
        print(f"✗ 端口转发操作失败: {e}")
    
    # 7. 获取日志
    print_section("7. 获取设备日志")
    
    print("\n获取设备日志 (前 500 个字符)...")
    try:
        logs = client.hilog()
        if logs:
            print(logs[:500])
            print("...")
        else:
            print("日志为空")
    except Exception as e:
        print(f"✗ 获取日志失败: {e}")
    
    # 8. 清理
    print_section("8. 清理")
    
    try:
        # 清理设备上的测试文件
        print("清理设备上的测试文件...")
        client.shell(f"rm -f {remote_path}")
        print("✓ 已清理设备文件")
        
        # 清理本地文件
        if os.path.exists(test_file):
            os.remove(test_file)
        if os.path.exists(received_file):
            os.remove(received_file)
        print("✓ 已清理本地文件")
    except Exception as e:
        print(f"清理时出错: {e}")
    
    print_section("测试完成")
    print("所有主要功能测试完成！")


if __name__ == "__main__":
    main()
