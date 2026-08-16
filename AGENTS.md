# AGENTS.md

## 项目概览

BiliAssist 是使用 Rust、GPUI 和 gpui-component 构建的 B站账号管理原生桌面应用，界面语言为中文。

## 技术栈

- 界面：GPUI + gpui-component
- 异步：Tokio
- 网络：reqwest
- 应用认证：Supabase Auth
- 存储：AES-256-GCM 加密本地文件

## 常用命令

从仓库根目录运行：

    cargo run --manifest-path src-tauri/Cargo.toml
    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo check --locked --manifest-path src-tauri/Cargo.toml
    cargo test --locked --manifest-path src-tauri/Cargo.toml
    cargo build --release --locked --manifest-path src-tauri/Cargo.toml

package.json 只保存发布版本并提供上述 Cargo 命令的 npm 别名，不包含 Web 前端依赖。

## 架构

- src-tauri/src/main.rs：原生二进制入口。
- src-tauri/src/lib.rs：初始化日志、本地存储、自动回复服务和 GPUI 应用。
- src-tauri/src/ui/app.rs：主窗口、导航、账号管理与自动回复界面。
- src-tauri/src/ui/auth.rs：Supabase 邮箱密码和 OTP 认证。
- src-tauri/src/ui/platform.rs：本地激活与开机自启。
- src-tauri/src/bilibili.rs：B站二维码登录。
- src-tauri/src/storage.rs：加密账号持久化。
- src-tauri/src/auto_reply/：视频评论、动态评论、私信和关注处理器。

GPUI 界面直接调用同一进程中的 Rust 模块，没有 WebView 或 IPC command 边界。

## 自动回复

- MsgSource 包含 Comment、Dynamic、DirectMessage 和 Follow。
- 视频评论处理器覆盖一级评论、子评论以及用户配置的指定 BV 视频。
- 每个渠道拥有独立回复内容和策略，视频与动态渠道支持自动点赞。
- 配置与历史保存在 auto_reply_settings.json，回复和点赞去重集合单独持久化。

## 发布

- .github/workflows/release.yml 根据 Conventional Commit 自动调整语义化版本。
- 版本在 package.json 和 src-tauri/Cargo.toml 中保持一致。
- Windows、macOS、Linux 均直接构建 Cargo 原生二进制。

## 数据

用户数据位于 ~/.bilibili_account_manager/。不要在开发或测试中删除 key.bin，也不要提交 Cookie、访问令牌或真实用户数据。
