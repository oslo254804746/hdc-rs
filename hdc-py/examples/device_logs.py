"""
设备日志示例
"""

from hdc_rs import HdcClient


def main():
    # 连接到 HDC 服务器和设备
    client = HdcClient("127.0.0.1:8710")
    devices = client.list_targets()
    
    if not devices:
        print("未找到设备")
        return
    
    client.connect_device(devices[0])
    print(f"已连接到设备: {devices[0]}")
    
    # 获取所有日志
    print("\n获取所有日志 (前 1000 个字符)...")
    try:
        logs = client.hilog()
        print(logs[:1000])
    except Exception as e:
        print(f"错误: {e}")
    
    # 使用标签过滤
    print("\n使用标签过滤日志...")
    try:
        # 根据实际情况修改标签
        logs = client.hilog("-t MyApp")
        print(logs)
    except Exception as e:
        print(f"错误: {e}")
    
    # 其他 hilog 选项示例
    print("\n其他 hilog 选项:")
    print("- 按级别过滤: client.hilog('-L D')  # Debug 级别")
    print("- 按域过滤: client.hilog('-D 0xD000')  # 特定域")
    print("- 组合过滤: client.hilog('-t MyApp -L I')  # MyApp 标签的 Info 级别")


if __name__ == "__main__":
    main()
