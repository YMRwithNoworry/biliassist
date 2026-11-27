# 贡献指南

## 开发环境

- Rust stable
- Git
- Linux 上所需的 GPUI 系统库

克隆后从仓库根目录运行：

    cargo check --locked --manifest-path src-tauri/Cargo.toml
    cargo run --manifest-path src-tauri/Cargo.toml

## 代码结构

    src-tauri/
    ├── Cargo.toml
    └── src/
        ├── ui/             # GPUI 原生界面、认证与平台能力
        ├── auto_reply/     # 自动回复处理器和持久状态
        ├── bilibili.rs     # B站扫码登录接口
        ├── storage.rs      # 本地加密账号存储
        ├── lib.rs          # 应用初始化
        └── main.rs         # 二进制入口

## 提交前检查

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
