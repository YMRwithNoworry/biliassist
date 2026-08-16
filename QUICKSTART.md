# 快速开始

## 环境

安装 Rust stable，并确认 rustc 和 cargo 命令可用。

Linux 需要安装 GPUI 使用的系统库。Ubuntu 22.04 可执行：

    sudo apt-get install gcc g++ libfontconfig-dev libglib2.0-dev libssl-dev libvulkan1 libwayland-dev libx11-xcb-dev libxkbcommon-x11-dev libzstd-dev

## 启动

Windows 可双击 start.bat。Linux/macOS 可执行：

    chmod +x start.sh
    ./start.sh

也可直接启动：

    cargo run --locked --manifest-path src-tauri/Cargo.toml

首次打开后，先使用邮箱登录应用，再进入“扫码登录”添加 B站账号。

## 自动回复

1. 在“账号管理”确认当前 B站账号。
2. 在“自动回复”打开总开关并设置检查间隔。
3. 分别配置视频评论、动态评论、私信和关注渠道。
4. 需要处理特定视频时，添加 BV 号并填写独立回复内容。
5. 保存后可使用“立即处理视频评论”或“立即处理动态评论”验证。

视频评论渠道会同时处理一级评论和子评论。回复记录保存在本地，并在处理结束后立即刷新。

## 构建

Windows 可运行 build.bat，全平台通用命令为：

    cargo build --release --locked --manifest-path src-tauri/Cargo.toml

产物位于 src-tauri/target/release/。

## 常见问题

- 扫码无响应：确认网络可访问 B站接口，二维码过期后重新生成。
- 自动回复未执行：确认总开关、当前渠道和当前 B站账号均已启用。
- 账号无法读取：检查 .bilibili_account_manager/key.bin 是否被移动或替换。
- Linux 窗口无法打开：确认 Vulkan 驱动和 Wayland/X11 运行库已经安装。
