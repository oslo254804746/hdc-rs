#!/bin/bash
# HDC-RS Python 绑定构建脚本 (Linux/macOS)

echo "================================"
echo "HDC-RS Python 绑定构建脚本"
echo "================================"

# 检查 Python
echo
echo "检查 Python..."
if command -v python3 &> /dev/null; then
    PYTHON_VERSION=$(python3 --version)
    echo "✓ 找到 Python: $PYTHON_VERSION"
    PYTHON=python3
elif command -v python &> /dev/null; then
    PYTHON_VERSION=$(python --version)
    echo "✓ 找到 Python: $PYTHON_VERSION"
    PYTHON=python
else
    echo "✗ 未找到 Python。请安装 Python 3.8 或更高版本。"
    exit 1
fi

# 检查 Rust
echo
echo "检查 Rust..."
if command -v cargo &> /dev/null; then
    RUST_VERSION=$(cargo --version)
    echo "✓ 找到 Rust: $RUST_VERSION"
else
    echo "✗ 未找到 Rust。请从 https://rustup.rs/ 安装 Rust。"
    exit 1
fi

# 检查 maturin
echo
echo "检查 maturin..."
if command -v maturin &> /dev/null; then
    MATURIN_VERSION=$(maturin --version)
    echo "✓ 找到 maturin: $MATURIN_VERSION"
else
    echo "! 未找到 maturin，正在安装..."
    $PYTHON -m pip install maturin
    if [ $? -ne 0 ]; then
        echo "✗ 安装 maturin 失败"
        exit 1
    fi
    echo "✓ maturin 安装成功"
fi

# 进入 pyo3 目录
echo
echo "进入 pyo3 目录..."
cd pyo3 || exit 1

# 清理之前的构建
echo
echo "清理之前的构建..."
if [ -d "target" ]; then
    rm -rf target
fi
echo "✓ 清理完成"

# 构建项目
echo
echo "构建 Python 扩展..."
echo "这可能需要几分钟时间..."

maturin develop --release

if [ $? -eq 0 ]; then
    echo
    echo "✓ 构建成功！"
    
    # 测试导入
    echo
    echo "测试导入模块..."
    $PYTHON -c "import hdc_rs; print('✓ 模块导入成功')"
    
    if [ $? -eq 0 ]; then
        echo
        echo "✓ 所有检查通过！"
        echo
        echo "可以运行示例:"
        echo "  $PYTHON examples/basic.py"
        echo "  $PYTHON examples/file_transfer.py"
        echo "  $PYTHON examples/comprehensive.py"
    else
        echo
        echo "✗ 模块导入失败"
    fi
else
    echo
    echo "✗ 构建失败"
    exit 1
fi

echo
echo "构建完成！"
