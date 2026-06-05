# ClipMan

<div align="center">

<img src="app-icon.png" alt="ClipMan" width="128" />

**轻量级现代剪切板管理器**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/Version-2.0.1-blue.svg)](https://github.com/RustyPiano/ClipMan/releases)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/RustyPiano/ClipMan)

[下载使用](#-安装) · [功能特性](#-功能) · [开发文档](#-开发)

[English](README_EN.md)

</div>

## 简介

ClipMan 是一个**轻量级**、**高性能**的现代剪切板管理器。采用 Rust + Svelte 5 技术栈，专注于提供核心实用功能。

**为什么选择 ClipMan？**

- ✅ **永久保存** - 重启系统后历史依然可用
- ✅ **置顶功能** - 常用内容一键置顶，随时调用
- ✅ **轻量快速** - 安装包体积小，启动迅速
- ✅ **现代界面** - 简洁美观，支持多主题
- ✅ **开源免费** - MIT 协议，完全开源

## ✨ 功能

### 核心特性

- **📌 置顶常用内容** - 一键置顶代码片段、命令、链接等，永久保存
- **💾 持久化存储** - SQLite 本地数据库，重启不丢失，智能去重
- **🔍 全文搜索** - SQLite FTS5 + trigram 索引，支持中英文实时搜索
- **⌨️ QuickBar 快捷取用** - 任意应用快速调出（默认 `Cmd/Ctrl+Shift+V`），键盘选择并按设置自动粘贴或仅复制
- **🎯 托盘菜单** - 快速访问最近和置顶内容
- **🎨 多主题** - 浅色/深色/淡粉色主题，跟随系统
- **🌐 多语言** - 支持中文和英文，自动检测系统语言

### 其他功能

- 🛡️ 跳过密码类剪切板内容（可选）
- 🔄 自动更新
- 🚀 开机自启动
- 📁 自定义存储位置
- 🧹 数据管理

## 📥 安装

访问 [GitHub Releases](https://github.com/RustyPiano/ClipMan/releases/latest) 下载：

- **Windows**: `ClipMan_2.0.1_x64_en-US.msi`
- **macOS (Apple Silicon)**: `ClipMan_2.0.1_aarch64.dmg`
- **macOS (Intel)**: `ClipMan_2.0.1_x64.dmg`
- **Linux**: `ClipMan_2.0.1_amd64.AppImage`

### macOS 权限

首次运行需授予辅助功能权限：

**系统设置** → **隐私与安全性** → **辅助功能** → 添加 ClipMan

## 🚀 使用

1. 使用快捷键 `Cmd+Shift+V` (macOS) / `Ctrl+Shift+V` (Windows) 打开
2. 所有复制内容自动保存在历史列表中
3. 点击 📌 图标置顶常用内容
4. 搜索框快速查找历史记录
5. 点击条目执行 QuickBar 默认取用行为：默认自动粘回当前应用；关闭自动粘贴后仅复制
6. 点击托盘图标快速访问（托盘菜单始终仅复制）

## 🛠️ 技术

**后端**

- Tauri 2.11 - 轻量桌面框架
- SQLite + FTS5 - 高性能数据库
- Rust - 内存安全，性能卓越

**前端**

- Svelte 5 - 现代响应式框架
- Tailwind CSS 4 - 现代样式方案
- Vite 8 - 快速构建工具

**性能定位**

- ⚡ 轻量弹窗：QuickBar 使用隐藏小窗口，按需唤起
- 💾 本地优先：SQLite 持久化，无后台网络服务
- 🔍 索引搜索：FTS5 + trigram 索引用于降低搜索延迟
- 📦 小体积分发：具体安装包大小以当前 Release 构建产物为准

## 🔧 开发

### 环境要求

- Bun 1.3+（推荐）或 Node.js 20.19+
- Rust 1.96.0（项目通过 `rust-toolchain.toml` 固定）
- 系统: Windows 10+ / macOS 10.13+ / Linux

### 快速开始

```bash
# 克隆项目
git clone https://github.com/RustyPiano/ClipMan.git
cd ClipMan

# 安装依赖
bun install

# 开发
bun tauri dev

# 构建
bun tauri build
```

### 项目结构

```
ClipMan/
├── src/              # Svelte 前端
│   ├── lib/
│   │   ├── components/   # UI 组件
│   │   ├── stores/       # 状态管理
│   │   └── i18n/         # 多语言
│   └── routes/           # 页面路由
├── src-tauri/        # Rust 后端
│   └── src/
│       ├── main.rs       # 入口、托盘
│       ├── clipboard.rs  # 剪切板监控
│       ├── storage.rs    # 数据库
│       ├── paste.rs      # 复制/自动粘贴
│       ├── window.rs     # QuickBar / 设置窗口
│       └── settings.rs   # 设置管理
└── package.json
```

## 🗺️ 路线图

**已完成**

- [x] 剪切板监控和历史
- [x] 置顶功能
- [x] 全文搜索
- [x] QuickBar 自动粘贴
- [x] 全局快捷键
- [x] 系统托盘
- [x] 自动更新
- [x] 多主题
- [x] 跳过密码类剪切板内容
- [x] 自定义存储位置
- [x] 开机自启动

**计划中**

- [ ] 智能图片压缩（AVIF/WebP/MozJPEG，可配置）
- [ ] 多设备同步
- [ ] 分组管理
- [ ] 规则过滤
- [ ] 插件系统
- [ ] 命令行工具

## 🤝 贡献

欢迎通过 Issue 或 Pull Request 贡献。

- 🐛 [报告 Bug](https://github.com/RustyPiano/ClipMan/issues)
- ✨ [功能建议](https://github.com/RustyPiano/ClipMan/issues)
- 💬 [讨论区](https://github.com/RustyPiano/ClipMan/discussions)

## 📄 许可证

[MIT License](LICENSE)

## 🙏 致谢

- [Tauri](https://tauri.app/) - 桌面应用框架
- [Svelte](https://svelte.dev/) - 前端框架
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite 绑定
- [arboard](https://github.com/1Password/arboard) - 剪切板库

---

<div align="center">

**如果觉得有帮助，请给个 ⭐️ Star！**

Made with ❤️ by [RustyPiano](https://github.com/RustyPiano)

</div>
