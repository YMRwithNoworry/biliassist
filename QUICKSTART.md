# 快速开始指南

## 前置条件

在开始之前，请确保已安装以下软件：

### 1. Node.js
下载安装：https://nodejs.org/

验证安装：
```bash
node --version
npm --version
```

### 2. Rust
下载安装：https://rustup.rs/

验证安装：
```bash
rustc --version
cargo --version
```

## 快速启动

### Windows 用户

双击运行 `start.bat`，或在命令行中执行：
```bash
start.bat
```

### Linux/macOS 用户

赋予执行权限并运行：
```bash
chmod +x start.sh
./start.sh
```

### 手动启动

如果启动脚本无法运行，可以手动执行：

```bash
# 1. 安装前端依赖
npm install

# 2. 启动开发服务器
npm run tauri dev
```

## 首次使用

### 1. 扫码登录
1. 启动应用后，点击"扫码登录"按钮
2. 使用 B站 App 扫描二维码
3. 在手机上确认登录
4. 登录成功后，账号信息会自动保存

### 2. 管理账号
1. 点击"管理账号"按钮
2. 查看所有已登录的账号
3. 点击"激活"切换当前使用的账号
4. 点击"删除"移除不需要的账号

### 3. 设置自动回复
1. 点击"设置回复"按钮
2. 勾选"启用自动回复"
3. 编辑回复内容（支持变量：{用户名}、{时间}）
4. 设置回复间隔（秒）
5. 选择是否每个用户只回复一次
6. 点击"测试回复"预览效果

## 构建发布版本

### Windows
```bash
build.bat
```

### Linux/macOS
```bash
npm run tauri build
```

构建完成后，安装包位于：
- Windows: `src-tauri\target\release\bundle\`
- Linux: `src-tauri/target/release/bundle/`
- macOS: `src-tauri/target/release/bundle/`

## 常见问题

### Q: 扫码后没有反应？
A: 请确保网络连接正常，并刷新页面重试。

### Q: 账号信息丢失？
A: 检查用户目录下的 `.bilibili_account_manager` 文件夹是否存在。

### Q: 自动回复不工作？
A: 确保已启用自动回复功能，并且应用保持运行状态。

### Q: 构建失败？
A: 检查 Rust 和 Node.js 版本是否符合要求，并确保网络连接正常。

## 数据备份

账号数据存储在：
- Windows: `C:\Users\<用户名>\.bilibili_account_manager\`
- macOS: `/Users/<用户名>/.bilibili_account_manager/`
- Linux: `/home/<用户名>/.bilibili_account_manager/`

建议定期备份此文件夹。

## 技术支持

如遇到问题，请：
1. 查看控制台错误信息
2. 检查日志文件
3. 提交 Issue 并附上详细的错误描述

## 更新日志

### v0.1.0 (2024-04-19)
- 初始版本发布
- 支持扫码登录
- 支持账号管理
- 支持自动回复
