# BiliAssist

BiliAssist 是使用 Rust、GPUI 和 [gpui-component](https://github.com/longbridge/gpui-component) 构建的 B站账号管理与自动回复原生桌面应用。界面直接由 GPU 渲染，不依赖浏览器 WebView、Vue 或 Tauri。

## 功能

- Supabase 邮箱密码、邮件验证码登录和 Plus 等级识别
- B站二维码登录，多账号加密保存、切换、删除与云端同步
- 自动回复视频评论、评论区子评论、动态评论、私信和关注事件
- 自动点赞视频与动态评论
- 按渠道配置固定回复、每条回复或每用户一次策略
- 添加指定 BV 视频，并为每个视频单独配置回复内容、策略与点赞
- 1 至 3600 秒检查间隔、立即处理评论和实时回复记录
- 开机自启与本地 Plus 激活

## 技术栈

- 原生界面：GPUI + gpui-component
- 异步运行时：Tokio
- 网络：reqwest
- 本地存储：AES-256-GCM
- 应用认证：Supabase Auth

## 下载与安装

在 [GitHub Releases](https://github.com/YMRwithNoworry/biliassist/releases) 下载对应系统的发布包：

- Windows 推荐使用 `windows-x86_64-setup.exe` 安装版；免安装使用 `windows-x86_64-portable.zip` 绿色版
- macOS 使用 `.dmg` 安装包，也可下载包含标准 `.app` 的便携版
- Linux 使用 `linux-*-portable.tar.gz` 绿色版，完整解压后运行

请完整解压绿色版，不要直接在压缩软件中运行程序。

## 开发

需要安装 Rust stable。Linux 还需要 Fontconfig、Wayland/X11、Vulkan Loader 等 GPUI 系统依赖。

    cargo run --locked --manifest-path src-tauri/Cargo.toml

也可使用兼容脚本：

    npm run dev

package.json 不包含 JavaScript 依赖，只提供版本号和 Cargo 命令别名。

## 检查与测试

    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo check --locked --manifest-path src-tauri/Cargo.toml
    cargo test --locked --manifest-path src-tauri/Cargo.toml

## 构建

    cargo build --release --locked --manifest-path src-tauri/Cargo.toml

Windows 输出位于 src-tauri/target/release/bilibili-account-manager.exe，macOS/Linux 输出位于 src-tauri/target/release/bilibili-account-manager。

## 数据存储

运行数据位于用户主目录下的 .bilibili_account_manager/：

- bilibili_accounts.enc：AES-256-GCM 加密的 B站账号
- key.bin：本地账号加密密钥
- auto_reply_settings.json：自动回复配置和历史
- replied_set.json：已回复去重记录
- liked_set.json：已点赞去重记录
- auth_session.json：应用登录会话

请勿删除或替换 key.bin，否则已有账号数据将无法解密。自动回复功能需要应用保持运行。

## License

MIT
