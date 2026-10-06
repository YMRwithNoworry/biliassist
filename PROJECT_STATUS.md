# 项目状态

## 已完成

- [x] 使用 Tauri 2 + React 重写桌面界面（Rust 业务逻辑不变，前端替换 GPUI）
- [x] 用 `#[tauri::command]` 暴露后端能力，前端统一经 src/lib/ipc.ts 调用
- [x] Supabase 邮箱密码与邮件验证码认证（在 Rust 侧调用）
- [x] B站二维码登录和多账号加密管理
- [x] 账号、自动回复配置和去重状态云同步
- [x] 视频一级评论与子评论自动回复
- [x] 动态评论、私信和关注自动回复
- [x] 视频与动态评论自动点赞
- [x] 指定 BV 视频及独立回复内容、策略和点赞配置
- [x] 1 秒起的轮询间隔与立即处理
- [x] 新评论秒级自动回复（边抓边回的快速通道，默认 3 秒）
- [x] 本地回复历史即时刷新
- [x] 开机自启、系统托盘和 Plus 权限
- [x] Windows、macOS、Linux 构建与打包流水线

## 技术栈

- Rust 2021
- Tauri 2（tray-icon、single-instance、autostart 插件）
- React 19 / TypeScript / Vite / Tailwind CSS v4 / shadcn/ui
- Tokio / reqwest
- Serde
- AES-256-GCM
- Supabase Auth REST API

## 目录

    src/                     # React 前端（Tailwind v4 + shadcn/ui）
    ├── App.tsx
    ├── index.css            # 设计令牌
    ├── components/ui/       # shadcn/ui 组件
    ├── lib/ipc.ts
    └── views/

    src-tauri/src/           # Rust 后端
    ├── commands.rs
    ├── auth.rs
    ├── cloud.rs
    ├── platform.rs
    ├── auto_reply/
    ├── bilibili.rs
    ├── storage.rs
    ├── lib.rs
    └── main.rs

## 验证基线

每次提交应通过 npm run build、npm run typecheck、cargo fmt、cargo check、cargo test 和 cargo build。发布构建必须使用 Cargo.lock 与 package-lock.json。
