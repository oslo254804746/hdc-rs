# 构建和测试说明

## 快速开始

### 1. 构建 Python 扩展

运行构建脚本（推荐）：

**Windows PowerShell:**
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

或者手动构建：
```bash
cd pyo3
pip install maturin
maturin develop --release
```

### 2. 运行测试

```bash
# 基础导入测试
python test_basic.py

# 运行示例
python examples/basic.py
python examples/comprehensive.py
```

## 构建成功后

如果构建成功，您应该能够在 Python 中导入并使用 hdc_rs：

```python
from hdc_rs import HdcClient

# 连接到 HDC 服务器
client = HdcClient("127.0.0.1:8710")

# 列出设备
devices = client.list_targets()
print(devices)
```

## 故障排除

### 如果构建失败

1. 确保 Rust 已安装：`rustup --version`
2. 更新 Rust：`rustup update`
3. 清理并重试：
   ```bash
   rm -rf target
   maturin develop --release
   ```

### 如果导入失败

1. 确保在正确的 Python 环境中
2. 重新运行：`maturin develop`
3. 检查构建输出是否有错误

### 如果连接失败

1. 确保 HDC 服务器正在运行
2. 验证端口：默认是 8710
3. 测试连接：在命令行运行 `hdc list targets`

## 开发工作流

1. 修改 Rust 代码（src/lib.rs）
2. 重新构建：`maturin develop`
3. 测试 Python 代码
4. 重复上述步骤

## 更多信息

- 完整文档: README.md
- 快速入门: QUICKSTART.md
- 示例代码: examples/ 目录
