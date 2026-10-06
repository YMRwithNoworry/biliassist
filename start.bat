@echo off
echo 正在启动 B站账号管理工具...
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

if not exist node_modules (
    echo 正在安装前端依赖...
    call npm install
    if %errorlevel% neq 0 (
        echo 安装依赖失败
        pause
        exit /b 1
    )
)

echo 环境检查通过
echo.

echo 启动 Tauri 应用...
call npm run tauri:dev
