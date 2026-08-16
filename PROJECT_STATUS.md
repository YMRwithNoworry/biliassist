# 项目状态

## 已完成

- [x] 使用 GPUI 和 gpui-component 重写原生桌面界面
- [x] 移除 Vue、Vite、Tauri 和 WebView 运行时
- [x] Supabase 邮箱密码与邮件验证码认证
- [x] B站二维码登录和多账号加密管理
- [x] 账号、自动回复配置和去重状态云同步
- [x] 视频一级评论与子评论自动回复
- [x] 动态评论、私信和关注自动回复
- [x] 视频与动态评论自动点赞
- [x] 指定 BV 视频及独立回复内容、策略和点赞配置
- [x] 1 秒起的轮询间隔与立即处理
- [x] 本地回复历史即时刷新
- [x] 开机自启和 Plus 权限
- [x] Windows、macOS、Linux 原生构建流水线

## 技术栈

- Rust 2021
- GPUI / gpui-component
- Tokio / reqwest
- Serde
- AES-256-GCM
- Supabase Auth REST API

## 目录

    src-tauri/src/
    ├── ui/
    │   ├── app.rs
    │   ├── auth.rs
    │   └── platform.rs
    ├── auto_reply/
    ├── bilibili.rs
    ├── storage.rs
    ├── lib.rs
    └── main.rs

## 验证基线

每次提交应通过 cargo fmt、cargo check、cargo test 和 cargo build。发布构建必须使用 Cargo.lock。
