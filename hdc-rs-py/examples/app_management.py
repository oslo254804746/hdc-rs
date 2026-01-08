"""
应用管理示例
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
    
    # 注意: 以下示例需要实际的 .hap 文件
    # 请根据实际情况修改文件路径和包名
    
    # 安装应用
    print("\n安装应用示例:")
    print("client.install(['app.hap'], replace=True)")
    # 取消注释以下行来实际执行
    # result = client.install(["path/to/app.hap"], replace=True)
    # print(f"结果: {result}")
    
    # 安装共享包
    print("\n安装共享包示例:")
    print("client.install(['shared.hsp'], shared=True)")
    # result = client.install(["path/to/shared.hsp"], shared=True)
    # print(f"结果: {result}")
    
    # 卸载应用
    print("\n卸载应用示例:")
    print("client.uninstall('com.example.app')")
    # 取消注释以下行来实际执行
    # result = client.uninstall("com.example.app")
    # print(f"结果: {result}")
    
    # 卸载应用但保留数据
    print("\n卸载应用但保留数据示例:")
    print("client.uninstall('com.example.app', keep_data=True)")
    # result = client.uninstall("com.example.app", keep_data=True)
    # print(f"结果: {result}")
    
    # 列出已安装的包
    print("\n列出已安装的包:")
    try:
        output = client.shell("bm dump -a")
        print(output)
    except Exception as e:
        print(f"错误: {e}")


if __name__ == "__main__":
    main()
