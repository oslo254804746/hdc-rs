# 🚀 开始使用 HDC-RS Python 绑定

欢迎！这个快速指南将帮助您在 5 分钟内开始使用 hdc-rs 的 Python 绑定。

## ⚡ 快速开始 (3 步)

### 1️⃣ 构建

```powershell
cd pyo3
.\build.ps1
```

### 2️⃣ 测试

```powershell
python test_basic.py
```

### 3️⃣ 运行示例

```python
# 创建 test.py
from hdc_rs import HdcClient

client = HdcClient("127.0.0.1:8710")
devices = client.list_targets()
print(f"设备: {devices}")
```

```powershell
python test.py
```

## 📚 主要功能

### 连接和设备管理
```python
client = HdcClient("127.0.0.1:8710")
devices = client.list_targets()
client.connect_device(devices[0])
```

### 执行命令
```python
output = client.shell("ls -l /data")
print(output)
```

### 文件传输
```python
# 发送
client.file_send("local.txt", "/data/local/tmp/remote.txt")

# 接收
client.file_recv("/data/local/tmp/remote.txt", "local.txt")
```

### 端口转发
```python
client.fport("tcp:8080", "tcp:8080")
```

### 应用管理
```python
client.install(["app.hap"], replace=True)
client.uninstall("com.example.app")
```

### 设备日志
```python
logs = client.hilog()
print(logs)
```

## 📖 完整文档

- **API 文档**: [README.md](README.md)
- **快速入门**: [QUICKSTART.md](QUICKSTART.md)
- **构建说明**: [BUILD.md](BUILD.md)
- **实现细节**: [IMPLEMENTATION.md](IMPLEMENTATION.md)

## 🎯 示例代码

在 `examples/` 目录中有完整的示例：

```powershell
python examples/basic.py           # 基础使用
python examples/file_transfer.py   # 文件传输
python examples/port_forward.py    # 端口转发
python examples/app_management.py  # 应用管理
python examples/device_logs.py     # 设备日志
python examples/comprehensive.py   # 完整示例
```

## ❓ 常见问题

**Q: 导入失败？**
A: 运行 `maturin develop`

**Q: 连接失败？**
A: 确保 HDC 服务器正在运行，端口正确（默认 8710）

**Q: 未找到设备？**
A: 确保设备已连接，USB 调试已启用

## 🛠️ 需要帮助？

1. 查看 [QUICKSTART.md](QUICKSTART.md) 了解详细步骤
2. 运行 `python test_basic.py` 诊断问题
3. 查看 [README.md](README.md) 了解完整 API

## ✨ 开始编码吧！

祝您使用愉快！🎉
