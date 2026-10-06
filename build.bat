@echo off
echo 正在构建 B站账号管理工具...
echo.

REM 检查 Node.js 与 Rust
where npm >nul 2>&1
if %errorlevel% neq 0 (
    echo 错误: 未找到 Node.js，请先安装 Node.js 20+
    pause
    exit /b 1
)

rustc --version >nul 2>&1
if %errorlevel% neq 0 (
    echo 错误: 未找到 Rust，请先安装 Rust
    pause
    exit /b 1
)

echo 环境检查通过
echo.

echo 构建 React 前端...
call npm install
call npm run build
if %errorlevel% neq 0 (
    echo 前端构建失败
    pause
    exit /b 1
)

echo 构建 Tauri 后端...
cargo build --release --locked --features custom-protocol --manifest-path src-tauri\Cargo.toml
if %errorlevel% neq 0 (
    echo 构建失败
    pause
    exit /b 1
)

echo.
echo 构建成功！
echo 程序位置: src-tauri	argeteleaseilibili-account-manager.exe
pause
