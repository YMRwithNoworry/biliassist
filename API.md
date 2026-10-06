# 内部 API

前端（React）与后端（Rust）通过 Tauri IPC 通信：所有能力都在 src-tauri/src/commands.rs 里以 `#[tauri::command]` 暴露，前端只能通过 src/lib/ipc.ts 的类型化封装 `api.*` 调用，界面层不直接使用 `invoke`。

## B站扫码登录

src-tauri/src/bilibili.rs，对应 `get_qr_code`、`generate_qr_code`、`check_login_status`：

- get_qr_code：请求 B站登录二维码，返回 `{ qrcode, qrcodeKey }`。
- generate_qr_code：把二维码内容渲染成 PNG 的 Base64 数据，供前端 `<img>` 直接显示。
- check_login_status：轮询二维码状态，成功后读取用户信息并保存账号。

扫码使用 B站 passport-login 接口，账号信息使用 web-interface/nav 接口确认。

## 账号存储

src-tauri/src/storage.rs，对应 `get_accounts`、`activate_account`、`delete_account`、`sync_accounts`：

- get_accounts：读取全部本地账号。
- activate_account：切换当前账号。
- delete_account：删除账号。
- get_active_account：返回自动回复使用的当前账号（仅内部使用）。
- sync_accounts：合并外部账号集合。

Account 包含 uid、name、cookie、active 和 createdAt。账号集合使用 AES-256-GCM 加密，Cookie 不得写入日志。

## 自动回复

src-tauri/src/auto_reply/mod.rs，对应 `get_auto_reply_settings`、`save_auto_reply_settings`、`manual_reply_video_comments`、`manual_reply_dynamic_comments`、`test_auto_reply`、`get_replied_set`、`get_liked_set`、`merge_replied_set`、`merge_liked_set`：

- get_settings / save_settings：读取或保存完整自动回复设置。
- manual_reply_comments：立即处理视频评论。
- manual_reply_dynamic_comments：立即处理动态评论。
- start_auto_reply_service：启动常驻轮询。
- get_replied_set / get_liked_set：读取去重集合。
- merge_replied_set / merge_liked_set：合并去重集合。

MsgSource 支持 Comment、Dynamic、DirectMessage 和 Follow。视频评论处理器还会遍历一级评论的子评论，并处理 tracked_videos 中配置的 BV 号。

## 应用账号与云同步

- src-tauri/src/auth.rs：`auth_restore_session`、`auth_sign_in`、`auth_send_otp`、`auth_verify_otp`、`auth_logout`，调用 Supabase Auth REST 接口，会话写入 auth_session.json。
- src-tauri/src/cloud.rs：`cloud_upload_all`、`cloud_download_all`，用当前会话同步账号、自动回复设置和去重集合。
- src-tauri/src/platform.rs：`is_licensed`、`activate_license`；开机自启由 tauri-plugin-autostart 在 `get_autostart_status` / `set_autostart` 中提供。

## 配置模型

AutoReplySettings 的主要字段：

- enabled：自动回复总开关。
- interval：完整检查间隔，单位为秒。
- fastInterval：快速通道间隔，单位为秒，1 至 60，默认 3。
- channels：视频评论、动态评论、私信、关注的独立设置。
- trackedVideos：指定 BV 视频及其独立回复设置。
- history：最近的回复记录。

回复策略为 perMessage 或 oncePerUser。固定文案支持 {用户名} 和 {时间} 变量。

## 本地文件

数据目录为用户主目录下的 .bilibili_account_manager/：

- bilibili_accounts.enc：加密账号数据。
- key.bin：AES-256 密钥。
- auto_reply_settings.json：回复配置与历史。
- replied_set.json：回复去重集合。
- liked_set.json：点赞去重集合。
- auth_session.json：Supabase 应用会话。
