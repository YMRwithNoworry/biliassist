#!/bin/bash

echo "正在启动 B站账号管理工具..."
echo

# 检查 Node.js
if ! command -v node &> /dev/null; then
    echo "错误: 未找到 Node.js，请先安装 Node.js"
    exit 1
fi

# 检查 Rust
if ! command -v rustc &> /dev/null; then
    echo "错误: 未找到 Rust，请先安装 Rust"
    exit 1
fi

echo "环境检查通过"
echo

# 安装前端依赖
if [ ! -d "node_modules" ]; then
    echo "安装前端依赖..."
    npm install
    if [ $? -ne 0 ]; then
        echo "安装依赖失败"
        exit 1
    fi
fi

echo "启动开发服务器..."
npm run tauri dev
