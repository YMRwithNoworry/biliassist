@echo off
echo 正在构建 B站账号管理工具...
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

echo 开始构建...
cargo build --release --locked --manifest-path src-tauri\Cargo.toml
if %errorlevel% neq 0 (
    echo 构建失败
    pause
    exit /b 1
)

echo.
echo 构建成功！
echo 程序位置: src-tauri\target\release\bilibili-account-manager.exe
pause
