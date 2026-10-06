# 开发说明

本仓库的当前架构、命令和持久化约定以 AGENTS.md 为准。

需要特别注意：

- 应用是 Tauri 2 + React 桌面程序：后端在 src-tauri/，前端在 src/，两者只能通过 src/lib/ipc.ts 的 `api.*` 与 src-tauri/src/commands.rs 里的命令通信。
- 界面代码位于 src/（views/ 页面、lib/ipc.ts 契约），后端业务逻辑位于 src-tauri/src/（auto_reply/ 自动回复，auth.rs/cloud.rs Supabase 认证与云同步）。
- 前端改动必须先 `npm run build`，否则 cargo 构建内嵌的仍是上一次的 dist/。
- 自动回复设置需要兼容已有 JSON，修改 Serde 模型时必须提供默认值。
- B站账号 Cookie 使用 AES-256-GCM 加密，禁止记录到日志或测试快照。
- 提交前运行 npm run build、npm run typecheck、cargo fmt、cargo check、cargo test，并使用锁文件构建。
