# ClipMan v1.0.0 - 首个正式版本 🎉

现代化的跨平台剪切板管理器,轻量、快速、安全。

## ✨ 核心特性

- 📋 **自动历史记录** - 保存所有复制的文本和图像
- 🔍 **智能搜索** - 快速查找历史内容
- 📌 **置顶功能** - 常用内容一键置顶,快速访问
- 🔒 **端到端加密** - AES-256-GCM 保护本地数据
- ⌨️ **全局热键** - `Cmd+Shift+V` / `Ctrl+Shift+V` 快速唤起
- 🎯 **系统托盘** - 菜单栏显示最近 20 条记录

## 📦 下载安装

### macOS
- **Apple Silicon (M1/M2/M3)**: `ClipMan_aarch64.dmg`
- **Intel Mac**: `ClipMan_x64.dmg`

### Windows
- **64位**: `ClipMan_x64_en-US.msi`

### Linux
- **Debian/Ubuntu**: `clipman_1.0.0_amd64.deb`
- **AppImage**: `clipman_1.0.0_amd64.AppImage`

## 🚀 快速开始

1. 安装后首次启动,授予剪切板访问权限
2. 使用热键 `Cmd+Shift+V` (macOS) 或 `Ctrl+Shift+V` (Windows/Linux) 打开
3. 点击 📍 图标可以置顶常用内容
4. 搜索框支持关键词快速查找

## 📊 性能指标

- ⚡ 启动时间: < 1s
- 💾 安装包大小: < 10MB
- 🧠 内存占用: < 50MB
- 🔧 支持 100+ 条历史记录

## ⚠️ 已知限制

- macOS 开发模式下会显示 Dock 图标 (Release 正常)
- 菜单栏暂不支持图片复制 (计划 v1.1)
- 首次运行需要授予系统权限

## 🛠️ 技术栈

- Rust 1.82+ + Tauri 2.0
- Svelte 5 + Tailwind CSS 4
- SQLite + FTS5 全文搜索

---

**完整文档**: [README.md](https://github.com/YOUR_USERNAME/ClipMan)
**问题反馈**: [GitHub Issues](https://github.com/YOUR_USERNAME/ClipMan/issues)

感谢使用 ClipMan! 🙏
