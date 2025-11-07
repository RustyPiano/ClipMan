# ClipMan v1.0.0 🎉

> 现代化的跨平台剪切板管理器 - 首个正式版本发布!

## ✨ 核心功能

### 📋 剪切板管理
- **自动历史记录** - 自动保存复制的文本和图像
- **智能搜索** - 快速搜索历史记录内容
- **置顶功能** - 将常用内容置顶,快速访问
- **端到端加密** - 使用 AES-256-GCM 加密保护本地数据

### 🎯 用户体验
- **系统托盘集成** - 菜单栏快速访问,显示最近 20 条记录
- **全局热键** - 默认 `Cmd+Shift+V` (macOS) / `Ctrl+Shift+V` (Windows/Linux)
- **沉浸式界面** - 基于 Svelte 5 的现代化 UI
- **跨平台支持** - 支持 macOS, Windows, Linux

### ⚙️ 技术特性
- **轻量级** - 安装包 < 10MB,内存占用 < 50MB
- **高性能** - Rust 后端 + SQLite 存储
- **快速启动** - < 1 秒启动时间
- **隐私优先** - 所有数据仅存储在本地,不联网

## 📦 安装方式

### macOS

**Apple Silicon (M1/M2/M3)**
```bash
# 下载并安装
curl -L https://github.com/YOUR_USERNAME/ClipMan/releases/download/v1.0.0/ClipMan_aarch64.dmg -o ClipMan.dmg
open ClipMan.dmg
```

**Intel Mac**
```bash
# 下载并安装
curl -L https://github.com/YOUR_USERNAME/ClipMan/releases/download/v1.0.0/ClipMan_x64.dmg -o ClipMan.dmg
open ClipMan.dmg
```

### Windows
1. 下载 `ClipMan_x64_en-US.msi`
2. 双击运行安装程序
3. 首次运行时授予剪切板访问权限

### Linux (Ubuntu/Debian)
```bash
# 下载 .deb 包
wget https://github.com/YOUR_USERNAME/ClipMan/releases/download/v1.0.0/clipman_1.0.0_amd64.deb

# 安装
sudo dpkg -i clipman_1.0.0_amd64.deb
```

## 🚀 快速开始

1. **首次启动** - 授予剪切板访问权限(macOS/Windows)
2. **使用热键** - 按 `Cmd+Shift+V` (或 `Ctrl+Shift+V`) 打开历史记录
3. **置顶内容** - 点击 📍 图标将常用内容置顶
4. **搜索历史** - 在搜索框输入关键词快速查找

## 📝 更新日志

### 新功能
- ✅ 剪切板历史记录自动保存
- ✅ 文本和图像支持
- ✅ 全文搜索功能
- ✅ 置顶常用内容
- ✅ 系统托盘菜单快速访问
- ✅ 全局热键支持
- ✅ AES-256 加密保护
- ✅ 清除非置顶记录功能
- ✅ 自动窗口同步

### 技术栈
- **后端**: Rust 1.82+ + Tauri 2.0
- **前端**: Svelte 5 + Tailwind CSS 4
- **数据库**: SQLite + FTS5 全文搜索
- **加密**: AES-256-GCM

## ⚠️ 已知问题

1. **macOS 开发模式** - Dev 模式下会显示 Dock 图标(Release 版本正常)
2. **图片复制** - 菜单栏图片复制功能尚未实现
3. **Windows 首次启动** - 可能需要手动授予剪切板权限

## 🔧 故障排除

### macOS 权限问题
```bash
# 在系统设置中授予辅助功能权限
系统设置 -> 隐私与安全性 -> 辅助功能 -> 添加 ClipMan
```

### 数据库错误
如果遇到解密错误,可以清理数据库:
```bash
# macOS/Linux
rm -rf ~/Library/Application\ Support/com.clipman.app/clips.db

# Windows
del %APPDATA%\com.clipman.app\clips.db
```

### 热键冲突
在设置页面可以自定义全局热键。

## 🤝 贡献

欢迎提交 Issue 和 Pull Request!

- 报告 Bug: [GitHub Issues](https://github.com/YOUR_USERNAME/ClipMan/issues)
- 功能建议: [GitHub Discussions](https://github.com/YOUR_USERNAME/ClipMan/discussions)

## 📄 开源协议

本项目采用 MIT 协议开源。

## 🙏 致谢

- [Tauri](https://tauri.app/) - 跨平台应用框架
- [Svelte](https://svelte.dev/) - 响应式前端框架
- [arboard](https://github.com/1Password/arboard) - 剪切板库

---

**首次发布版本** - 如有问题请及时反馈! 🚀
