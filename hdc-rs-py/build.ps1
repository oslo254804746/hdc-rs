# HDC-RS Python 绑定构建脚本

Write-Host "HDC-RS Python 绑定构建脚本" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan

# 检查 Python
Write-Host "`n检查 Python..." -ForegroundColor Yellow
if (Get-Command python -ErrorAction SilentlyContinue) {
    $pythonVersion = python --version
    Write-Host "✓ 找到 Python: $pythonVersion" -ForegroundColor Green
} else {
    Write-Host "✗ 未找到 Python。请安装 Python 3.8 或更高版本。" -ForegroundColor Red
    exit 1
}

# 检查 Rust
Write-Host "`n检查 Rust..." -ForegroundColor Yellow
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    $rustVersion = cargo --version
    Write-Host "✓ 找到 Rust: $rustVersion" -ForegroundColor Green
} else {
    Write-Host "✗ 未找到 Rust。请从 https://rustup.rs/ 安装 Rust。" -ForegroundColor Red
    exit 1
}

# 检查 maturin
Write-Host "`n检查 maturin..." -ForegroundColor Yellow
if (Get-Command maturin -ErrorAction SilentlyContinue) {
    $maturinVersion = maturin --version
    Write-Host "✓ 找到 maturin: $maturinVersion" -ForegroundColor Green
} else {
    Write-Host "! 未找到 maturin，正在安装..." -ForegroundColor Yellow
    python -m pip install maturin
    if ($LASTEXITCODE -ne 0) {
        Write-Host "✗ 安装 maturin 失败" -ForegroundColor Red
        exit 1
    }
    Write-Host "✓ maturin 安装成功" -ForegroundColor Green
}

# 进入 pyo3 目录
Write-Host "`n进入 pyo3 目录..." -ForegroundColor Yellow
Set-Location pyo3

# 清理之前的构建
Write-Host "`n清理之前的构建..." -ForegroundColor Yellow
if (Test-Path target) {
    Remove-Item -Recurse -Force target
}
Write-Host "✓ 清理完成" -ForegroundColor Green

# 构建项目
Write-Host "`n构建 Python 扩展..." -ForegroundColor Yellow
Write-Host "这可能需要几分钟时间..." -ForegroundColor Cyan

maturin develop --release

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n✓ 构建成功！" -ForegroundColor Green
    
    # 测试导入
    Write-Host "`n测试导入模块..." -ForegroundColor Yellow
    python -c "import hdc_rs_py; print('✓ 模块导入成功')"
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "`n✓ 所有检查通过！" -ForegroundColor Green
        Write-Host "`n可以运行示例:" -ForegroundColor Cyan
        Write-Host "  python examples/basic.py" -ForegroundColor White
        Write-Host "  python examples/file_transfer.py" -ForegroundColor White
        Write-Host "  python examples/comprehensive.py" -ForegroundColor White
    } else {
        Write-Host "`n✗ 模块导入失败" -ForegroundColor Red
    }
} else {
    Write-Host "`n✗ 构建失败" -ForegroundColor Red
    exit 1
}

Write-Host "`n构建完成！" -ForegroundColor Cyan
