#!/bin/bash

echo "正在启动 B站账号管理工具..."
echo

# 检查 Rust
if ! command -v rustc &> /dev/null; then
    echo "错误: 未找到 Rust，请先安装 Rust"
    exit 1
fi

echo "环境检查通过"
echo

echo "启动 GPUI 原生应用..."
cargo run --locked --manifest-path src-tauri/Cargo.toml
