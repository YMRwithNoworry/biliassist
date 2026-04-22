@echo off
echo 正在启动 B站账号管理工具...
echo.

REM 检查 Node.js
node --version >nul 2>&1
if %errorlevel% neq 0 (
    echo 错误: 未找到 Node.js，请先安装 Node.js
    pause
    exit /b 1
)

REM 检查 Rust
rustc --version >nul 2>&1
if %errorlevel% neq 0 (
    echo 错误: 未找到 Rust，请先安装 Rust
    pause
    exit /b 1
)

echo 环境检查通过
echo.

REM 安装前端依赖
if not exist "node_modules" (
    echo 安装前端依赖...
    call npm install
    if %errorlevel% neq 0 (
        echo 安装依赖失败
        pause
        exit /b 1
    )
)

echo 启动开发服务器...
call npm run tauri dev
