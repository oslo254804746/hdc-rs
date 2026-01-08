"""
端口转发示例
"""

from hdc_rs_py import HdcClient


def main():
    # 连接到 HDC 服务器和设备
    client = HdcClient("127.0.0.1:8710")
    devices = client.list_targets()
    
    if not devices:
        print("未找到设备")
        return
    
    client.connect_device(devices[0])
    print(f"已连接到设备: {devices[0]}")
    
    # 创建 TCP 端口转发 (local:8080 -> device:8080)
    print("\n创建端口转发: local:8080 -> device:8080")
    result = client.fport("tcp:8080", "tcp:8080")
    print(f"结果: {result}")
    
    # 创建反向端口转发 (device:9090 -> local:9090)
    print("\n创建反向端口转发: device:9090 -> local:9090")
    result = client.rport("tcp:9090", "tcp:9090")
    print(f"结果: {result}")
    
    # Unix socket 转发示例
    print("\n创建 Unix socket 转发")
    try:
        result = client.fport("tcp:8888", "localabstract:myapp")
        print(f"结果: {result}")
    except Exception as e:
        print(f"错误: {e}")
    
    # 移除端口转发
    print("\n移除端口转发")
    try:
        result = client.fport_remove("tcp:8080 tcp:8080")
        print(f"结果: {result}")
    except Exception as e:
        print(f"错误: {e}")


if __name__ == "__main__":
    main()
