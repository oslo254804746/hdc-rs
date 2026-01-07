# HDC-RS Python API 快速参考

## 导入
```python
from hdc_rs import HdcClient
```

## 初始化
```python
client = HdcClient("127.0.0.1:8710")
```

## 设备管理

| 方法 | 说明 | 示例 |
|------|------|------|
| `list_targets()` | 列出所有设备 | `devices = client.list_targets()` |
| `connect_device(id)` | 连接到设备 | `client.connect_device("FMR...")` |
| `wait_for_device()` | 等待设备连接 | `device = client.wait_for_device()` |

## Shell 命令

| 方法 | 说明 | 示例 |
|------|------|------|
| `shell(cmd)` | 执行命令 | `output = client.shell("ls -l")` |

## 文件传输

| 方法 | 说明 | 示例 |
|------|------|------|
| `file_send(local, remote, ...)` | 发送文件 | `client.file_send("a.txt", "/data/a.txt")` |
| `file_recv(remote, local, ...)` | 接收文件 | `client.file_recv("/data/a.txt", "b.txt")` |

**选项**: `compress`, `hold_timestamp`, `sync_mode`, `mode_sync`

## 端口转发

| 方法 | 说明 | 示例 |
|------|------|------|
| `fport(local, remote)` | 端口转发 | `client.fport("tcp:8080", "tcp:8080")` |
| `rport(remote, local)` | 反向转发 | `client.rport("tcp:9090", "tcp:9090")` |
| `fport_remove(task)` | 移除转发 | `client.fport_remove("tcp:8080 tcp:8080")` |

**节点格式**: `tcp:PORT`, `localfilesystem:PATH`, `jdwp:PID`, `ark:PID@TID@Debugger`

## 应用管理

| 方法 | 说明 | 示例 |
|------|------|------|
| `install(pkgs, ...)` | 安装应用 | `client.install(["app.hap"], replace=True)` |
| `uninstall(pkg, ...)` | 卸载应用 | `client.uninstall("com.example.app")` |

**安装选项**: `replace`, `shared`  
**卸载选项**: `keep_data`, `shared`

## 日志

| 方法 | 说明 | 示例 |
|------|------|------|
| `hilog(args)` | 获取日志 | `logs = client.hilog()` |
| | 带过滤 | `logs = client.hilog("-t MyTag")` |

## 完整示例

```python
from hdc_rs import HdcClient

# 连接
client = HdcClient("127.0.0.1:8710")

# 列出并连接设备
devices = client.list_targets()
if devices:
    client.connect_device(devices[0])
    
    # 执行命令
    print(client.shell("pwd"))
    
    # 文件传输
    client.file_send("local.txt", "/data/local/tmp/remote.txt")
    
    # 端口转发
    client.fport("tcp:8080", "tcp:8080")
    
    # 获取日志
    logs = client.hilog()
    print(logs[:500])
```

## 错误处理

```python
try:
    client = HdcClient("127.0.0.1:8710")
    devices = client.list_targets()
except RuntimeError as e:
    print(f"错误: {e}")
```

## 更多信息

- 完整文档: [README.md](README.md)
- 入门指南: [START_HERE.md](START_HERE.md)
- 示例代码: `examples/` 目录
