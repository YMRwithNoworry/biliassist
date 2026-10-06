#!/bin/bash
set -e

echo "正在启动 B站账号管理工具..."
echo

if ! command -v npm &> /dev/null; then
    echo "错误: 未找到 Node.js，请先安装 Node.js 20+"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "错误: 未找到 Rust，请先安装 Rust"
    exit 1
fi

if [ ! -d node_modules ]; then
    echo "正在安装前端依赖..."
    npm install
fi

echo "环境检查通过"
echo

echo "启动 Tauri 应用..."
npm run tauri:dev
