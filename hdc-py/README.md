# HDC-RS Python 绑定

这是 hdc-rs 的 Python 绑定，提供了 HarmonyOS Device Connector (HDC) 客户端的 Python 接口。

## 安装

### 从源码构建

首先确保已安装 Rust 和 Python 3.8+：

```bash
# 安装 maturin
pip install maturin

# 进入 pyo3 目录
cd pyo3

# 开发模式安装（推荐用于开发）
maturin develop

# 或者构建 wheel 包
maturin build --release

# 安装构建好的 wheel
pip install target/wheels/hdc_rs-*.whl
```

## 快速开始

```python
from hdc_rs import HdcClient

# 连接到 HDC 服务器
client = HdcClient("127.0.0.1:8710")

# 列出所有设备
devices = client.list_targets()
print(f"设备列表: {devices}")

if devices:
    # 连接到第一个设备
    client.connect_device(devices[0])
    
    # 执行 shell 命令
    output = client.shell("ls -l /data")
    print(output)
```

## API 文档

### HdcClient

#### `__init__(addr: str)`

创建新的 HDC 客户端并连接到服务器。

- `addr`: 服务器地址，例如 `"127.0.0.1:8710"`

```python
client = HdcClient("127.0.0.1:8710")
```

#### `list_targets() -> list[str]`

列出所有已连接的设备。

```python
devices = client.list_targets()
print(devices)  # ['FMR0223C13000649']
```

#### `connect_device(device_id: str)`

连接到指定设备。

```python
client.connect_device("FMR0223C13000649")
```

#### `shell(command: str) -> str`

在设备上执行 shell 命令。

```python
output = client.shell("ls -l /data")
print(output)
```

#### `file_send(local_path: str, remote_path: str, compress: bool = False, preserve_timestamp: bool = False) -> str`

发送文件到设备。

- `local_path`: 本地文件路径
- `remote_path`: 设备上的远程路径
- `compress`: 是否压缩传输（默认：False）
- `preserve_timestamp`: 是否保留时间戳（默认：False）

```python
result = client.file_send("local.txt", "/data/local/tmp/remote.txt")
print(result)
```

#### `file_recv(remote_path: str, local_path: str, compress: bool = False, preserve_timestamp: bool = False) -> str`

从设备接收文件。

- `remote_path`: 设备上的远程路径
- `local_path`: 本地文件路径
- `compress`: 是否压缩传输（默认：False）
- `preserve_timestamp`: 是否保留时间戳（默认：False）

```python
result = client.file_recv("/data/local/tmp/remote.txt", "local.txt")
print(result)
```

#### `fport(local: str, remote: str) -> str`

创建端口转发（本地 -> 设备）。

- `local`: 本地转发节点，例如 `"tcp:8080"`
- `remote`: 远程转发节点，例如 `"tcp:8080"`

支持的节点格式：
- `tcp:port` - TCP 端口
- `localfilesystem:path` - 本地文件系统 Unix socket
- `localreserved:name` - 本地保留 Unix socket
- `localabstract:name` - 本地抽象 Unix socket
- `jdwp:pid` - JDWP 进程（仅远程）
- `ark:pid@tid@Debugger` - Ark 调试器（仅远程）

```python
result = client.fport("tcp:8080", "tcp:8080")
print(result)
```

#### `rport(remote: str, local: str) -> str`

创建反向端口转发（设备 -> 本地）。

```python
result = client.rport("tcp:9090", "tcp:9090")
print(result)
```

#### `fport_remove(task_str: str) -> str`

移除端口转发。

```python
result = client.fport_remove("tcp:8080 tcp:8080")
print(result)
```

#### `install(packages: list[str], replace: bool = False, shared: bool = False) -> str`

安装应用程序。

- `packages`: 包文件路径列表（.hap 或 .hsp 文件）
- `replace`: 替换现有应用（默认：False）
- `shared`: 为多应用安装共享包（默认：False）

```python
result = client.install(["app.hap"], replace=True)
print(result)
```

#### `uninstall(package: str, keep_data: bool = False, shared: bool = False) -> str`

卸载应用程序。

- `package`: 包名
- `keep_data`: 保留数据和缓存目录（默认：False）
- `shared`: 移除共享包（默认：False）

```python
result = client.uninstall("com.example.app")
print(result)
```

#### `hilog(args: str | None = None) -> str`

获取设备日志。

- `args`: 可选的 hilog 参数，例如 `"-t MyTag"`

```python
# 获取所有日志
logs = client.hilog()
print(logs)

# 使用过滤器
logs = client.hilog("-t MyTag")
print(logs)
```

#### `wait_for_device() -> str`

等待设备连接。此方法会阻塞，直到有设备连接。

```python
device_id = client.wait_for_device()
print(f"设备已连接: {device_id}")
```

## 示例

### 完整示例

```python
from hdc_rs import HdcClient

def main():
    # 连接到 HDC 服务器
    client = HdcClient("127.0.0.1:8710")
    
    # 列出设备
    devices = client.list_targets()
    print(f"可用设备: {devices}")
    
    if not devices:
        print("未找到设备")
        return
    
    # 连接到第一个设备
    device_id = devices[0]
    client.connect_device(device_id)
    print(f"已连接到设备: {device_id}")
    
    # 执行 shell 命令
    print("\n执行 shell 命令...")
    output = client.shell("ls -l /data/local/tmp")
    print(output)
    
    # 文件传输
    print("\n发送文件...")
    result = client.file_send("test.txt", "/data/local/tmp/test.txt")
    print(result)
    
    print("\n接收文件...")
    result = client.file_recv("/data/local/tmp/test.txt", "received.txt")
    print(result)
    
    # 端口转发
    print("\n设置端口转发...")
    result = client.fport("tcp:8080", "tcp:8080")
    print(result)
    
    # 获取日志
    print("\n获取设备日志...")
    logs = client.hilog()
    print(logs[:500])  # 打印前 500 个字符

if __name__ == "__main__":
    main()
```

### 应用管理示例

```python
from hdc_rs import HdcClient

client = HdcClient("127.0.0.1:8710")
devices = client.list_targets()
client.connect_device(devices[0])

# 安装应用
print("安装应用...")
result = client.install(["app.hap"], replace=True)
print(result)

# 卸载应用
print("卸载应用...")
result = client.uninstall("com.example.app", keep_data=False)
print(result)
```

### 设备监控示例

```python
from hdc_rs import HdcClient
import time

def monitor_devices():
    client = HdcClient("127.0.0.1:8710")
    
    while True:
        devices = client.list_targets()
        print(f"当前设备: {devices}")
        time.sleep(5)

if __name__ == "__main__":
    monitor_devices()
```

## 开发

### 运行测试

```bash
# 在开发模式下构建并安装
maturin develop

# 运行 Python 测试
python -m pytest tests/
```

### 构建发布版本

```bash
# 构建 release 版本
maturin build --release

# 构建并发布到 PyPI
maturin publish
```

## 许可证

本项目采用双许可证：

- Apache License 2.0
- MIT License

详见 LICENSE-APACHE 和 LICENSE-MIT 文件。
