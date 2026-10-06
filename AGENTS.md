# AGENTS.md

## 项目概览

BiliAssist 是使用 Tauri 2 和 React 构建的 B站账号管理桌面应用：Rust 提供业务逻辑与系统集成，React 提供界面，界面语言为中文。

## 技术栈

- 界面：React 19 + TypeScript + Vite（src/）+ Tailwind CSS v4 + shadcn/ui（new-york）
- UI 组件：shadcn 原件在 src/components/ui/，应用级封装在 src/components/，图标统一 lucide-react，路由用 react-router-dom 的 HashRouter
- 桌面壳：Tauri 2（src-tauri/），前端只能通过 src/lib/ipc.ts 里的 `api.*` 调用 `#[tauri::command]`
- 异步：Tokio（tauri::async_runtime）
- 网络：reqwest
- 应用认证：Supabase Auth（在 Rust 侧调用，密钥不进前端）
- 存储：AES-256-GCM 加密本地文件

## 常用命令

从仓库根目录运行：

    npm install
    npm run tauri:dev
    npm run build
    npm run typecheck
    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo check --locked --manifest-path src-tauri/Cargo.toml
    cargo test --locked --manifest-path src-tauri/Cargo.toml
    cargo build --release --locked --features custom-protocol --manifest-path src-tauri/Cargo.toml

tauri-build 会把 dist/ 内嵌进二进制，因此改完前端必须先 `npm run build`，否则 cargo 构建的是上一次的前端产物。

## 架构

- src/main.tsx、src/App.tsx：React 入口与应用外壳（导航、登录守卫、激活状态）。
- src/lib/ipc.ts：唯一的 IPC 契约，前端所有后端调用都经过它。
- src/index.css：Tailwind 入口与全部设计令牌。
- src/components/ui/：shadcn/ui 组件源码，需要时直接改，不要另起一套。
- src/components/：应用级封装（ViewShell、SectionCard、StatCard、EmptyState、StatusBar、Dialog、Sidebar、ThemeToggle）。
- src/views/：登录、概览、账号管理、自动回复、支持项目等页面。
- src-tauri/src/main.rs：二进制入口。
- src-tauri/src/lib.rs：Tauri Builder、插件、托盘、关闭窗口隐藏和服务启动。
- src-tauri/src/commands.rs：全部 `#[tauri::command]`，是前端唯一能触达的后端边界。
- src-tauri/src/auth.rs：Supabase 邮箱密码和 OTP 认证。
- src-tauri/src/cloud.rs：账号与自动回复设置的云端同步。
- src-tauri/src/platform.rs：本地激活与开发模式判断；开机自启由 tauri-plugin-autostart 提供。
- src-tauri/src/bilibili.rs：B站二维码登录。
- src-tauri/src/storage.rs：加密账号持久化。
- src-tauri/src/auto_reply/：视频评论、动态评论、私信和关注处理器。

新增后端能力时先在 commands.rs 暴露命令，再在 src/lib/ipc.ts 补上类型化封装，界面层不直接调用 `invoke`。

## 主题与 UI 约定

- 主题状态在 src/state/theme.tsx，支持 light / dark / system，默认跟随系统，选择写入 localStorage 的 biliassist-theme。
- index.html 里有一段内联脚本，在首屏渲染前就把 <html data-theme> 设好，避免夜间模式刷新时闪一帧白屏。
- 所有颜色只能来自 src/index.css 的语义令牌（浅色在 :root，夜间在 [data-theme='dark']），Tailwind 类名形如 bg-card、text-muted-foreground、border-border、bg-brand。
- 组件里禁止写死颜色（bg-white、text-white、#xxx、bg-[#...]）；品牌红底上的文字用 text-brand-foreground，否则夜间模式会糊。
- 页面骨架统一用 ViewShell（固定页头 + 可滚动内容区），分组用 SectionCard，统计用 StatCard，空态用 EmptyState，行内提示用 StatusBar，对话框用 @/components/Dialog。
- 图标统一 lucide-react；瞬时反馈用 sonner 的 toast，需要用户留意的持久错误用 StatusBar。
- 新增依赖前先确认 src/components/ui 里的 shadcn 原件不够用；shadcn 组件是源码，可以直接改。

## 自动回复

- MsgSource 包含 Comment、Dynamic、DirectMessage 和 Follow。
- 视频评论处理器覆盖一级评论、子评论以及用户配置的指定 BV 视频。
- 每个渠道拥有独立回复内容和策略，视频与动态渠道支持自动点赞。
- 评论抓取是流式的（边抓边回）：抓到一页就立刻把消息送进通道并回复，不再等整轮扫描结束。
- 服务循环分两档：快速通道每 fastInterval 秒只抓每个评论目标的最新一页，负责秒回新评论；
  interval 仍控制完整补扫、私信和关注。私信、关注只走完整通道。
- fastInterval 默认 3 秒，范围 1 至 60，配置写入 auto_reply_settings.json；旧配置缺少该字段时取默认值。
- 快速通道对"子评论翻页"设有预算（每个目标 10 条线程），避免热门视频请求过密。
- 配置与历史保存在 auto_reply_settings.json，回复和点赞去重集合单独持久化。

## 提交与推送

- 改动通过 npm run build、cargo fmt、cargo check、cargo test 后，直接提交并推送到 origin main，不需要再向用户确认。
- 提交信息使用 Conventional Commits；release workflow 按最后一条提交信息决定版本号
  （feat 提升 minor，fix/chore/docs 提升 patch，带 `!` 或 BREAKING CHANGE 提升 major）。
- 推送会触发发布构建，因此推送前必须确认 Cargo.lock 已同步、package-lock.json 已提交且 `--locked` 构建可通过。

## 发布

- .github/workflows/release.yml 根据 Conventional Commit 自动调整语义化版本。
- 版本在 package.json、src-tauri/Cargo.toml 和 src-tauri/tauri.conf.json 中保持一致。
- 发布流程先执行 npm ci 与 npm run build 生成前端产物，再构建 Cargo 二进制，最后打包各平台安装包。
- 生产构建必须带 `--features custom-protocol`：否则 Tauri 不会把 dist/ 内嵌进二进制，窗口会去加载 build.devUrl（http://localhost:1420）并报“localhost 拒绝连接”。

## 数据

用户数据位于 ~/.bilibili_account_manager/。不要在开发或测试中删除 key.bin，也不要提交 Cookie、访问令牌或真实用户数据。

<!-- BEGIN dsh-global-system-prompt -->
<!--
  Managed by the dsh-global-system-prompt plugin. The rules below are synced from:
  C:\Users\Administrator\.dsh\global-prompt.md
  Edits inside this block are overwritten on the next sync; delete the block,
  markers included, to stop syncing.
-->

Global rules — follow these in every task in this project:

默认使用 Niubash（Bash on Windows，`niu -c`）执行任务。
每次任务结束将项目上传至github远程仓库，若无远程仓库则自己创建。
jdk在这里D:\MC\jdk
中文回答问题

<!-- END dsh-global-system-prompt -->
