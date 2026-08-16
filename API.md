# 内部 API

GPUI 界面与业务逻辑运行在同一 Rust 进程中，界面直接调用模块函数，不存在 WebView IPC 命令。

## B站扫码登录

src-tauri/src/bilibili.rs：

- get_qr_code：请求 B站登录二维码并返回 PNG 的 Base64 数据。
- check_login_status：轮询二维码状态，成功后读取用户信息并保存账号。

扫码使用 B站 passport-login 接口，账号信息使用 web-interface/nav 接口确认。

## 账号存储

src-tauri/src/storage.rs：

- get_accounts：读取全部本地账号。
- activate_account：切换当前账号。
- delete_account：删除账号。
- get_active_account：返回自动回复使用的当前账号。
- sync_accounts：合并外部账号集合。

Account 包含 uid、name、cookie、active 和 created_at。账号集合使用 AES-256-GCM 加密，Cookie 不得写入日志。

## 自动回复

src-tauri/src/auto_reply/mod.rs：

- get_settings / save_settings：读取或保存完整自动回复设置。
- manual_reply_comments：立即处理视频评论。
- manual_reply_dynamic_comments：立即处理动态评论。
- start_auto_reply_service：启动常驻轮询。
- get_replied_set / get_liked_set：读取去重集合。
- merge_replied_set / merge_liked_set：合并去重集合。

MsgSource 支持 Comment、Dynamic、DirectMessage 和 Follow。视频评论处理器还会遍历一级评论的子评论，并处理 tracked_videos 中配置的 BV 号。

## 配置模型

AutoReplySettings 的主要字段：

- enabled：自动回复总开关。
- interval：检查间隔，单位为秒。
- channels：视频评论、动态评论、私信、关注的独立设置。
- tracked_videos：指定 BV 视频及其独立回复设置。
- history：最近的回复记录。

回复策略为 PerMessage 或 OncePerUser。固定文案支持 {用户名} 和 {时间} 变量。

## 本地文件

数据目录为用户主目录下的 .bilibili_account_manager/：

- bilibili_accounts.enc：加密账号数据。
- key.bin：AES-256 密钥。
- auto_reply_settings.json：回复配置与历史。
- replied_set.json：回复去重集合。
- liked_set.json：点赞去重集合。
- auth_session.json：Supabase 应用会话。
