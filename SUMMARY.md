# 项目摘要

BiliAssist 是 Tauri 2 + React 桌面应用。Rust 侧负责窗口、托盘、业务逻辑和系统集成，React 侧负责界面，两者通过 src-tauri/src/commands.rs 的 `#[tauri::command]` 与 src/lib/ipc.ts 的类型化封装通信；Tokio 后台服务持续处理 B站事件。

应用包含两层账号：

- Supabase 应用账号控制登录状态与 Basic/Plus 等级。
- B站账号通过二维码添加，Cookie 使用本地 AES-256-GCM 加密。

自动回复按视频评论、动态评论、私信和关注四个渠道独立配置。视频评论同时覆盖一级评论、子评论和用户指定的 BV 视频；指定视频可单独设置回复文案、策略和自动点赞。所有渠道共用完整补扫间隔 interval 与快速通道间隔 fastInterval，并将成功记录及时写回界面和本地文件。

前端使用 Vite 构建到 dist/，再由 tauri-build 内嵌进二进制，因此构建顺序是先 `npm run build` 再 `cargo build`。
