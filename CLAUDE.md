# 开发说明

本仓库的当前架构、命令和持久化约定以 AGENTS.md 为准。

需要特别注意：

- 应用是纯 Rust GPUI 原生程序，不存在 Vue、Vite、Tauri 或 WebView 层。
- 界面状态位于 src-tauri/src/ui/，后台自动回复状态位于 src-tauri/src/auto_reply/。
- 自动回复设置需要兼容已有 JSON，修改 Serde 模型时必须提供默认值。
- B站账号 Cookie 使用 AES-256-GCM 加密，禁止记录到日志或测试快照。
- 提交前运行 cargo fmt、cargo check、cargo test，并使用锁文件构建。
