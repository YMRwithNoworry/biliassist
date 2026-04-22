# 贡献指南

感谢您对 B站账号管理工具的关注！

## 开发环境设置

### 前置要求
- Node.js 18+
- Rust 1.70+
- npm 或 yarn

### 安装步骤

1. 克隆仓库
```bash
git clone <repository-url>
cd 视频播放器
```

2. 安装依赖
```bash
npm install
```

3. 启动开发服务器
```bash
npm run tauri dev
```

## 项目结构

```
视频播放器/
├── src/                 # 前端源码
│   ├── views/          # 页面组件
│   ├── router/         # 路由配置
│   └── style.css       # 全局样式
├── src-tauri/          # Rust 后端
│   ├── src/
│   │   ├── main.rs     # 主入口
│   │   ├── bilibili.rs # B站 API
│   │   ├── storage.rs  # 数据存储
│   │   └── auto_reply.rs # 自动回复
│   └── Cargo.toml      # Rust 依赖
└── package.json        # 前端依赖
```

## 开发规范

### 代码风格
- 前端：遵循 Vue 3 官方风格指南
- Rust：使用 rustfmt 格式化代码

### 提交规范
使用语义化提交信息：
- `feat:` 新功能
- `fix:` 修复 bug
- `docs:` 文档更新
- `style:` 代码格式调整
- `refactor:` 重构
- `test:` 测试相关
- `chore:` 构建/工具相关

## 测试

在提交 PR 前，请确保：
1. 代码通过 linting
2. 功能测试通过
3. 没有引入新的 bug

## 问题报告

报告问题时，请提供：
- 复现步骤
- 预期行为
- 实际行为
- 环境信息（操作系统、版本等）

## 许可证

MIT
