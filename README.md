# ClipMan - 现代化剪切板管理器

> 跨平台剪切板管理器（Windows/macOS/Linux），使用 Rust + Tauri 2.0 + Svelte 5 构建

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.82+-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue.svg)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-red.svg)](https://svelte.dev/)

## ✨ 特性

- 🔄 **事件驱动剪切板监控** - 基于系统事件的即时捕获（v1.5.0+），CPU 占用接近 0%
- ⚡ **异步图像处理** - 后台处理图片，不阻塞主线程（v1.6.0+）
- 🔍 **强大搜索功能** - 基于 SQLite FTS5 全文搜索
- 📌 **置顶常用内容** - 快速访问常用剪切板项
- 🎯 **可配置托盘菜单** - 自定义显示项数量（3-10 置顶，10-50 最近）
- ⌨️ **全局热键** - `Cmd+Shift+V` (macOS) / `Ctrl+Shift+V` (Windows/Linux)
- 🎨 **现代化 UI** - 简洁易用，支持暗色模式
- 🔐 **隐私保护** - 本地存储，AES-256-GCM 端到端加密
- 🔄 **自动更新** - GitHub Releases 集成，自动检测新版本
- ⚡ **高性能** - Rust 后端，内存占用 < 50MB
- 🪶 **轻量级** - 安装包 < 5MB
- 🍎 **macOS 优化** - 菜单栏专属模式，无 Dock 图标

## 🛠️ 技术栈

### 后端
- **Rust 1.82+** - 安全、高性能系统编程语言
- **Tauri 2.0** - 现代桌面应用框架，WebView2 渲染
- **SQLite + FTS5** - 本地数据库，全文搜索支持
- **clipboard-master** - 事件驱动剪切板监控
- **arboard** - 跨平台剪切板读写操作
- **ring 0.17** - AES-256-GCM 加密
- **image 0.25** - 图像处理和缩略图生成

### 前端
- **Svelte 5** - 响应式框架，使用最新 Runes API (`$state`, `$derived`)
- **TypeScript** - 类型安全
- **Tailwind CSS 4** - 现代化样式
- **Vite 6** - 快速构建工具

## 🚀 快速开始

### 前置要求

- **Node.js** 18+ 或 Bun
- **Rust** 1.82+ (安装: https://rustup.rs/)
- **系统要求**:
  - Windows 10+ (需要 WebView2)
  - macOS 10.13+
  - Linux (需要 WebKit2GTK)

### 安装依赖

```bash
# 克隆仓库
git clone https://github.com/yourusername/clipman.git
cd clipman

# 安装依赖 (推荐使用 bun)
bun install
```

### 开发模式

```bash
# 启动开发服务器
bun tauri dev
```

**注意 (macOS)**: Dev 模式下可能显示 Dock 图标，这是正常的。Build 版本会正确隐藏 Dock 图标。

### 构建应用

```bash
# 构建生产版本
bun tauri build

# 构建产物位置:
# - Windows: src-tauri/target/release/bundle/msi/
# - macOS: src-tauri/target/release/bundle/dmg/
# - Linux: src-tauri/target/release/bundle/appimage/
```

### macOS 权限设置

首次运行时，需要授予剪切板访问权限：

1. 打开 **系统设置** → **隐私与安全性**
2. 选择 **辅助功能**
3. 点击 🔒 解锁
4. 添加 ClipMan（或运行它的终端/IDE）
5. 勾选启用

## 📁 项目结构

```
ClipMan/
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs         # 入口点、Tauri 设置、系统托盘、IPC 命令
│   │   ├── clipboard.rs    # 剪切板监控核心
│   │   ├── storage.rs      # SQLite 数据库封装（CRUD + FTS5）
│   │   ├── crypto.rs       # AES-256-GCM 加密模块
│   │   └── settings.rs     # 设置管理
│   ├── Cargo.toml          # Rust 依赖
│   └── tauri.conf.json     # Tauri 配置
├── src/                    # Svelte 前端
│   ├── lib/
│   │   ├── components/     # Svelte 组件
│   │   │   ├── SearchBar.svelte          # 搜索栏
│   │   │   ├── ClipboardItem.svelte      # 历史项卡片
│   │   │   ├── PermissionCheck.svelte    # macOS 权限检查 UI
│   │   │   └── Toast.svelte              # 通知提示
│   │   └── stores/
│   │       ├── clipboard.svelte.ts       # 剪切板状态管理（Runes）
│   │       └── router.svelte.ts          # 客户端路由
│   ├── routes/
│   │   ├── +page.svelte                  # 主页面
│   │   └── settings/+page.svelte         # 设置页面
│   ├── app.css             # 全局样式
│   └── main.ts             # 前端入口
├── package.json            # npm 依赖
├── vite.config.js          # Vite 配置
└── svelte.config.js        # Svelte 配置
```

## 🤝 贡献指南

欢迎贡献！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'feat: Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

**提交信息规范**: 使用 [Conventional Commits](https://www.conventionalcommits.org/)

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

- [Tauri](https://tauri.app/)
- [Svelte](https://svelte.dev/)
- [rusqlite](https://github.com/rusqlite/rusqlite)
- [arboard](https://github.com/1Password/arboard)
- [ring](https://github.com/briansmith/ring)

---

**注**: 本项目使用 2025 年最新技术栈构建，遵循现代化软件工程最佳实践。
