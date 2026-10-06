# 贡献指南

## 开发环境

- Rust stable
- Node.js 20+（前端构建与 Tauri CLI）
- Git
- Linux 上所需的 Tauri 系统库（WebKitGTK 4.1、GTK3、librsvg 等）

克隆后从仓库根目录运行：

    npm install
    npm run tauri:dev
    cargo check --locked --manifest-path src-tauri/Cargo.toml

## 代码结构

    src/                    # React 前端
    ├── App.tsx             # 外壳、导航与登录守卫
    ├── lib/ipc.ts          # IPC 契约
    └── views/              # 页面

    src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    └── src/
        ├── commands.rs     # 全部 #[tauri::command]
        ├── auth.rs         # Supabase 认证
        ├── cloud.rs        # 云同步
        ├── platform.rs     # 激活与开发模式判断
        ├── auto_reply/     # 自动回复处理器和持久状态
        ├── bilibili.rs     # B站扫码登录接口
        ├── storage.rs      # 本地加密账号存储
        ├── lib.rs          # Tauri Builder、托盘与初始化
        └── main.rs         # 二进制入口

## 提交前检查

    npm run build
    npm run typecheck
    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo check --locked --manifest-path src-tauri/Cargo.toml
    cargo test --locked --manifest-path src-tauri/Cargo.toml

使用 Conventional Commits：

- feat: 新功能
- fix: 缺陷修复
- docs: 文档
- refactor: 重构
- test: 测试
- chore: 构建与维护

涉及真实 B站账号的调试不得提交 Cookie、会话令牌或本地数据文件。报告问题时请附上操作系统、应用版本、复现步骤、预期行为和实际行为。

## 许可证

MIT
