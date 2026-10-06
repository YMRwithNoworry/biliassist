# BiliAssist

BiliAssist 是使用 Tauri 2 与 React 构建的 B站账号管理与自动回复桌面应用：Rust 负责业务逻辑、存储与系统集成，React 负责界面，两者通过 Tauri IPC command 通信。

## 功能

- Supabase 邮箱密码、邮件验证码登录和 Plus 等级识别
- B站二维码登录，多账号加密保存、切换、删除与云端同步
- 自动回复视频评论、评论区子评论、动态评论、私信和关注事件
- 新评论秒级自动回复：默认每 3 秒检查一次各视频与动态的最新评论，抓到即回
- 自动点赞视频与动态评论
- 日间/夜间模式切换，默认跟随系统主题
- 关闭窗口后继续在系统托盘运行，可从托盘恢复或退出
- 按渠道配置固定回复、每条回复或每用户一次策略
- 添加指定 BV 视频，并为每个视频单独配置回复内容、策略与点赞
- 1 至 3600 秒检查间隔、立即处理评论和实时回复记录
- 开机自启与本地 Plus 激活

## 技术栈

- 界面：React 19 + TypeScript + Vite（`src/`）
- 桌面壳与 IPC：Tauri 2（`src-tauri/`，`#[tauri::command]` 暴露后端能力）
- 异步运行时：Tokio
- 网络：reqwest
- 本地存储：AES-256-GCM
- 应用认证：Supabase Auth（在 Rust 侧调用，前端不接触密钥）

## 下载与安装

在 [GitHub Releases](https://github.com/YMRwithNoworry/biliassist/releases) 下载对应系统的发布包：

- Windows 推荐使用 `windows-x86_64-setup.exe` 安装版；免安装使用 `windows-x86_64-portable.zip` 绿色版
- macOS 使用 `.dmg` 安装包，也可下载包含标准 `.app` 的便携版
- Linux 使用 `linux-*-portable.tar.gz` 绿色版，完整解压后运行

请完整解压绿色版，不要直接在压缩软件中运行程序。

## 开发

需要 Node.js 20+ 与 Rust stable。Linux 还需要 WebKitGTK 4.1、GTK3、librsvg 等 Tauri 系统依赖。

    npm install
    npm run tauri:dev

`npm run dev` 只启动 Vite 开发服务器（浏览器里可以调样式，但 IPC 不可用）。

## 检查与测试

    npm run typecheck
    npm run build
    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo check --locked --manifest-path src-tauri/Cargo.toml
    cargo test --locked --manifest-path src-tauri/Cargo.toml

## 构建

    npm run build
    cargo build --release --locked --features custom-protocol --manifest-path src-tauri/Cargo.toml

前端产物在 `dist/`，会被编译进二进制；`--features custom-protocol` 必须保留，否则二进制不会内嵌前端，运行时会去连 http://localhost:1420。Windows 输出位于 src-tauri/target/release/bilibili-account-manager.exe，macOS/Linux 输出位于 src-tauri/target/release/bilibili-account-manager。

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
