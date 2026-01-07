# HDC-RS Python 快速入门

欢迎使用 HDC-RS 的 Python 绑定！本指南将帮助您快速开始使用。

## 前置要求

- Python 3.8 或更高版本
- Rust 和 Cargo (从 https://rustup.rs/ 安装)
- HarmonyOS 设备或模拟器
- HDC 服务器正在运行

## 安装步骤

### 方法 1: 使用构建脚本（推荐）

**Windows (PowerShell):**
```powershell
cd pyo3
.\build.ps1
```

**Linux/macOS:**
```bash
cd pyo3
chmod +x build.sh
./build.sh
```

### 方法 2: 手动构建

```bash
# 安装 maturin
pip install maturin

# 进入 pyo3 目录
cd pyo3

# 开发模式构建（推荐用于开发和测试）
maturin develop

# 或者构建 release 版本
maturin develop --release
```

### 方法 3: 构建 wheel 包

```bash
cd pyo3

# 构建 wheel 包
maturin build --release

# 安装生成的 wheel
pip install target/wheels/hdc_rs-*.whl
```

## 验证安装

在 Python 中测试导入：

```python
import hdc_rs
print("HDC-RS 安装成功！")
```

## 第一个程序

创建 `my_first_hdc.py`：

```python
from hdc_rs import HdcClient

# 连接到 HDC 服务器
client = HdcClient("127.0.0.1:8710")

# 列出所有设备
devices = client.list_targets()
print(f"找到设备: {devices}")

if devices:
    # 连接到第一个设备
    client.connect_device(devices[0])
    
    # 执行命令
    output = client.shell("ls -l /data")
    print(output)
```

运行：
```bash
python my_first_hdc.py
```

## 运行示例

我们提供了多个示例程序：

```bash
# 基础使用
python examples/basic.py

# 文件传输
python examples/file_transfer.py

# 端口转发
python examples/port_forward.py

# 应用管理
python examples/app_management.py

# 设备日志
python examples/device_logs.py

# 完整示例（展示所有功能）
python examples/comprehensive.py
```

## 常见问题

### Q: "ModuleNotFoundError: No module named 'hdc_rs'"

**A:** 确保已经运行 `maturin develop` 或安装了 wheel 包。

### Q: "无法连接到 HDC 服务器"

**A:** 检查：
1. HDC 服务器是否正在运行
2. 端口是否正确（默认 8710）
3. 防火墙设置

### Q: "未找到设备"

**A:** 确保：
1. 设备已通过 USB 连接
2. 设备已启用开发者模式和 USB 调试
3. 运行 `hdc list targets` 命令验证设备连接

### Q: 构建失败

**A:** 尝试：
1. 更新 Rust: `rustup update`
2. 清理构建: `rm -rf target`
3. 重新构建: `maturin develop --release`

## 开发技巧

### 开发模式

在开发过程中，使用 `maturin develop` 可以快速重新编译和安装：

```bash
# 修改代码后
cd pyo3
maturin develop
```

### 调试

在 Python 中捕获异常：

```python
from hdc_rs import HdcClient

try:
    client = HdcClient("127.0.0.1:8710")
    devices = client.list_targets()
    # ...
except Exception as e:
    print(f"错误: {e}")
    import traceback
    traceback.print_exc()
```

### 性能优化

- 使用 `--release` 标志构建以获得最佳性能
- 文件传输时考虑使用 `compress=True`
- 重用 client 对象而不是重复创建

## 下一步

- 阅读完整 API 文档: [README.md](README.md)
- 查看更多示例: `examples/` 目录
- 贡献代码: 欢迎提交 PR！

## 获取帮助

- GitHub Issues: https://github.com/your-repo/hdc-rs/issues
- 文档: [docs/](../docs/)

祝您使用愉快！🚀
