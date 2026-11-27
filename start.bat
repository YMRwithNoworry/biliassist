@echo off
echo 正在启动 B站账号管理工具...
echo.

REM 检查 Rust
rustc --version >nul 2>&1
if %errorlevel% neq 0 (
    echo 错误: 未找到 Rust，请先安装 Rust
    pause
    exit /b 1
)

echo 环境检查通过
echo.

echo 启动 GPUI 原生应用...
cargo run --locked --manifest-path src-tauri\Cargo.toml
