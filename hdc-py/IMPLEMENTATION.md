# HDC-RS Python 绑定实现总结

本文档总结了为 hdc-rs 创建的 Python 绑定实现。

## 项目结构

```
pyo3/
├── Cargo.toml           # Rust 项目配置
├── pyproject.toml       # Python 项目配置
├── README.md            # 完整的 API 文档
├── QUICKSTART.md        # 快速入门指南
├── BUILD.md             # 构建说明
├── .gitignore           # Git 忽略文件
├── build.ps1            # Windows 构建脚本
├── build.sh             # Linux/macOS 构建脚本
├── test_basic.py        # 基础测试脚本
├── src/
│   └── lib.rs           # Python 绑定实现（主文件）
└── examples/            # Python 示例
    ├── basic.py                  # 基础使用
    ├── file_transfer.py          # 文件传输
    ├── port_forward.py           # 端口转发
    ├── app_management.py         # 应用管理
    ├── device_logs.py            # 设备日志
    └── comprehensive.py          # 完整示例
```

## 已实现的功能

### 核心功能

✅ **客户端连接**
- `HdcClient(addr)` - 连接到 HDC 服务器

✅ **设备管理**
- `list_targets()` - 列出所有设备
- `connect_device(device_id)` - 连接到指定设备
- `wait_for_device()` - 等待设备连接

✅ **Shell 命令**
- `shell(command)` - 执行 shell 命令

✅ **文件传输**
- `file_send(local_path, remote_path, ...)` - 发送文件到设备
- `file_recv(remote_path, local_path, ...)` - 从设备接收文件
- 支持选项: compress, hold_timestamp, sync_mode, mode_sync

✅ **端口转发**
- `fport(local, remote)` - 创建端口转发
- `rport(remote, local)` - 创建反向端口转发
- `fport_remove(task_str)` - 移除端口转发
- 支持格式: tcp, localfilesystem, localreserved, localabstract, jdwp, ark

✅ **应用管理**
- `install(packages, replace, shared)` - 安装应用
- `uninstall(package, keep_data, shared)` - 卸载应用

✅ **设备日志**
- `hilog(args)` - 获取设备日志

### API 特性

- ✅ 完整的类型注解
- ✅ 详细的文档字符串
- ✅ 错误处理和异常转换
- ✅ 可选参数支持
- ✅ Python 风格的 API 设计

## 构建和使用

### 快速开始

```bash
# 进入目录
cd pyo3

# 使用构建脚本
./build.ps1  # Windows
./build.sh   # Linux/macOS

# 或手动构建
pip install maturin
maturin develop --release
```

### 使用示例

```python
from hdc_rs import HdcClient

# 连接
client = HdcClient("127.0.0.1:8710")

# 列出设备
devices = client.list_targets()

# 连接设备
client.connect_device(devices[0])

# 执行命令
output = client.shell("ls -l /data")

# 文件传输
client.file_send("local.txt", "/data/local/tmp/remote.txt")

# 端口转发
client.fport("tcp:8080", "tcp:8080")

# 获取日志
logs = client.hilog()
```

## 文件说明

### 核心文件

**src/lib.rs**
- Python 绑定的主实现
- 使用 PyO3 包装 hdc-rs 的 blocking API
- 所有公共方法都有详细文档

**Cargo.toml**
- Rust 依赖配置
- 设置为 cdylib (C 动态库)
- 包含 pyo3 和 hdc-rs 依赖

**pyproject.toml**
- Python 项目元数据
- Maturin 构建配置
- 包信息和分类

### 文档文件

**README.md**
- 完整的 API 文档
- 所有方法的详细说明
- 使用示例

**QUICKSTART.md**
- 快速入门指南
- 安装步骤
- 常见问题解答

**BUILD.md**
- 构建和测试说明
- 故障排除指南

### 示例文件

**examples/basic.py**
- 基础功能演示
- 设备列表和 shell 命令

**examples/file_transfer.py**
- 文件发送和接收
- 压缩和时间戳选项

**examples/port_forward.py**
- 端口转发配置
- TCP 和 Unix socket

**examples/app_management.py**
- 应用安装和卸载
- 选项配置示例

**examples/device_logs.py**
- 日志获取
- 过滤选项

**examples/comprehensive.py**
- 完整功能演示
- 所有 API 的使用示例

### 工具脚本

**build.ps1 / build.sh**
- 自动化构建脚本
- 环境检查
- 一键构建和测试

**test_basic.py**
- 基础功能测试
- 验证安装是否成功

## 技术实现细节

### PyO3 绑定

使用 PyO3 0.25 创建 Python 扩展模块：

```rust
#[pyclass]
struct HdcClient {
    inner: RustHdcClient,
}

#[pymethods]
impl HdcClient {
    #[new]
    fn new(addr: &str) -> PyResult<Self> { ... }
    
    fn list_targets(&mut self) -> PyResult<Vec<String>> { ... }
    // ...
}

#[pymodule]
fn hdc_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HdcClient>()?;
    Ok(())
}
```

### 错误处理

所有 Rust 错误都被转换为 Python 的 `RuntimeError`：

```rust
.map_err(|e| PyRuntimeError::new_err(e.to_string()))
```

### 参数映射

Rust 结构体映射为 Python 关键字参数：

```rust
#[pyo3(signature = (local_path, remote_path, compress=false, hold_timestamp=false))]
fn file_send(&mut self, ...) -> PyResult<String> {
    let options = RustFileTransferOptions::new()
        .compress(compress)
        .hold_timestamp(hold_timestamp);
    // ...
}
```

## 测试

### 运行测试

```bash
# 基础测试
python test_basic.py

# 运行所有示例
python examples/basic.py
python examples/comprehensive.py
```

### 测试覆盖

- ✅ 模块导入
- ✅ 客户端创建
- ✅ 设备列表
- ✅ Shell 命令执行
- ✅ 文件传输
- ✅ 端口转发
- ✅ 应用管理
- ✅ 日志获取

## 未来改进

可能的增强功能：

1. **异步支持**
   - 添加异步 API 版本
   - 使用 PyO3-asyncio

2. **类型提示**
   - 生成 .pyi 存根文件
   - 更好的 IDE 支持

3. **性能优化**
   - 使用 Python 的 buffer protocol
   - 减少数据拷贝

4. **更多选项**
   - 支持更多 HDC 命令
   - 添加流式 API

5. **打包发布**
   - 构建预编译 wheel
   - 发布到 PyPI

## 依赖要求

### 构建时
- Rust 1.70+
- Python 3.8+
- Maturin 1.9+

### 运行时
- Python 3.8+
- HDC 服务器（HarmonyOS SDK）

## 许可证

与主项目相同：
- Apache License 2.0
- MIT License

## 贡献

欢迎贡献！请：
1. Fork 项目
2. 创建特性分支
3. 提交 Pull Request

## 支持

- GitHub Issues
- 文档: [README.md](README.md)
- 示例: `examples/` 目录
