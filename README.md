# ClipMan - 轻量级现代剪切板管理器

<div align="center">

<img src="app-icon.png" alt="ClipMan" width="128" />

**一个真正懂你需求的剪切板工具**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/Version-1.9.0-blue.svg)](https://github.com/Kiaana/ClipMan/releases)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/Kiaana/ClipMan)

[下载使用](#-快速开始) · [功能特性](#-核心功能) · [开发文档](#-开发指南)

</div>

---

## 💡 为什么要开发 ClipMan？

在日常工作中，你是否也遇到过这些烦恼：

- ❌ **Windows 自带剪切板历史重启就没了** - 好不容易复制的代码、命令、链接，重启电脑后全部清空
- ❌ **没有置顶功能** - 那些经常用的 Git 命令、SSH 地址、常用文本，每次都要重新找、重新复制
- ❌ **现有工具太臃肿** - 安装包几十 MB，启动慢，占内存，功能一大堆却找不到最需要的
- ❌ **界面老旧难用** - 设计停留在十年前，暗色模式支持差，操作不流畅

**ClipMan 就是为了解决这些痛点而生的。**

这是一个**轻量级**（安装包不到 5MB）、**高性能**（内存占用小于 50MB）、**现代化**（Rust + Svelte 5 技术栈）的剪切板管理器，专注于提供**最核心、最实用的功能**。

---

## ✨ 核心功能

### 📌 置顶常用内容
**这是 ClipMan 最核心的功能！** 将你常用的内容（代码片段、SSH 地址、Git 命令、工作邮箱等）置顶，随时调用，告别重复复制粘贴。

- 一键置顶/取消置顶
- 置顶项永久保存，重启不丢失
- 支持文本、图片、富文本等多种格式

### 💾 持久化存储
所有剪切板历史都保存在本地数据库中，**重启系统后依然可用**。

- 基于 SQLite 数据库，稳定可靠
- 支持自定义数据存储位置（v1.8.0+）
- 一键迁移数据文件，自由选择存储路径
- **智能去重**（v1.9.0+）：复制已存在内容时自动移至顶部，不产生重复
- **实时数量管理**（v1.9.0+）：超过历史上限时自动清理最旧的非置顶项

### 🔍 强大的搜索功能
使用 SQLite FTS5 全文搜索，快速找到历史记录中的任何内容。

- 支持中文、英文等多语言搜索
- 实时搜索，输入即显示结果
- 搜索防抖优化，流畅不卡顿

### ⌨️ 全局快捷键
设置全局热键（默认 `Cmd+Shift+V` macOS / `Ctrl+Shift+V` Windows），任何应用中都能快速调出 ClipMan。

- **全新的智能录入系统**（v1.8.0）：直接按键盘组合即可设置快捷键
- 支持自定义任意组合键
- 提供常用快捷键快速预设

### 🎯 智能托盘菜单
在系统托盘中快速访问最近复制的内容和置顶项。

- 可自定义显示数量（置顶项 3-10 个，最近项 10-50 个）
- 点击即可复制到剪切板
- macOS 支持菜单栏模式，无 Dock 图标干扰

### 🎨 现代化界面
简洁美观的用户界面,支持多种主题模式。

- 基于 Svelte 5 和 Tailwind CSS 4 构建
- 支持浅色、深色、淡粉色主题和跟随系统
- 流畅的动画和交互体验
- 适配不同屏幕尺寸
- **优化的历史列表**（v1.9.0+）：历史记录严格按时间倒序，最新内容总在顶部

### 🔒 隐私与安全
所有数据都存储在本地，不会上传到任何服务器。

- 支持 AES-256-GCM 端到端加密（可选）
- 敏感内容可随时删除
- 完全离线运行，无网络权限

### 🔄 自动更新
内置自动更新功能，始终使用最新版本。

- 基于 GitHub Releases
- 可选择是否自动安装

### 🚀 开机自启动
设置后开机自动运行，静默启动到托盘，随时待命。

### 🧹 数据管理
- 清空所有历史记录
- 单独删除特定条目
- 在文件管理器中打开数据目录

---

## 🏆 为什么选择 ClipMan？

| 对比项 | Windows 自带 | 其他剪切板工具 | **ClipMan** |
|--------|-------------|--------------|------------|
| **持久化存储** | ❌ 重启丢失 | ✅ 部分支持 | ✅ **永久保存** |
| **置顶功能** | ❌ 不支持 | ❌ 很少支持 | ✅ **核心功能** |
| **安装包大小** | - | 20-100MB | ✅ **< 5MB** |
| **内存占用** | - | 100-300MB | ✅ **< 50MB** |
| **界面设计** | 简陋 | 老旧 | ✅ **现代化** |
| **启动速度** | - | 缓慢 | ✅ **秒开** |
| **全文搜索** | ❌ | 部分支持 | ✅ **FTS5 引擎** |
| **跨平台** | ❌ | 部分支持 | ✅ **Win/Mac/Linux** |
| **开源免费** | - | ❌ 收费/闭源 | ✅ **MIT 协议** |

---

## 🎉 最新更新 (v1.9.0)

### 🚀 性能优化
- **传输效率提升**：后端直接发送 Base64 编码数据，大幅减少 JSON 序列化开销
- **前端性能优化**：无需再进行 `btoa` 转换，显著降低主线程阻塞
- **图片处理更快**：处理大型图片数据时响应速度明显提升

### 📊 用户体验改进
- **更智能的历史排序**：历史记录页面严格按时间倒序，最新内容始终在顶部
- **实时去重机制**：复制重复内容时自动移至列表顶部，不再产生重复条目
- **自动数量管理**：超过历史上限时，最旧的非置顶项会自动移除
- **置顶项独立访问**：置顶内容在专门的"置顶"标签页中快速访问

> 详细更新内容请查看 [v1.9.0 Release Notes](https://github.com/Kiaana/ClipMan/releases/tag/v1.9.0)

---

## 🚀 快速开始

### 📥 下载安装

访问 [GitHub Releases](https://github.com/Kiaana/ClipMan/releases/latest) 下载最新版本：

- **Windows**: `ClipMan_1.9.0_x64_en-US.msi`
- **macOS (Apple Silicon)**: `ClipMan_1.9.0_aarch64.dmg`
- **macOS (Intel)**: `ClipMan_1.9.0_x64.dmg`
- **Linux**: `ClipMan_1.9.0_amd64.AppImage`

### ⚙️ macOS 权限设置

首次运行时，macOS 需要授予辅助功能权限：

1. 打开 **系统设置** → **隐私与安全性** → **辅助功能**
2. 点击 🔒 解锁
3. 添加 ClipMan 并勾选启用

### 🎯 基本使用

1. **启动应用** - 双击图标或使用快捷键 `Cmd+Shift+V` (macOS) / `Ctrl+Shift+V` (Windows) 打开
2. **查看历史** - 所有复制的内容会自动保存在主界面中
3. **置顶常用内容** - 点击任意条目的"📌"图标即可置顶
4. **搜索内容** - 在搜索框中输入关键词快速查找
5. **复制使用** - 点击任意条目即可复制到剪切板
6. **托盘快捷访问** - 点击系统托盘图标快速访问最近和置顶的内容

### 🎨 自定义设置

点击主界面右上角的"⚙️ 设置"按钮，可以配置：

- **常规设置**：开机自启、全局快捷键
- **外观设置**：主题选择（浅色/深色/淡粉色/跟随系统）
- **剪切板设置**：最大历史数量、图片质量、清空历史
- **托盘菜单**：显示数量、文本长度
- **数据存储**：自定义存储位置、数据迁移

---

## 🛠️ 技术架构

ClipMan 使用 2025 年最新的技术栈构建，性能和开发体验兼具：

### 后端（Rust）
- **Tauri 2.0** - 现代桌面应用框架，比 Electron 更轻量
- **SQLite + FTS5** - 高性能本地数据库，支持全文搜索
- **clipboard-master** - 事件驱动的剪切板监控，CPU 占用接近 0%
- **ring** - 军用级 AES-256-GCM 加密
- **image** - 高效的图像处理库
- **Base64 编码优化**（v1.9.0+）- 后端直接发送 Base64 数据，减少 JSON 序列化开销

### 前端（TypeScript + Svelte）
- **Svelte 5** - 最新的响应式框架，使用 Runes API
- **Tailwind CSS 4** - 现代化样式解决方案
- **Vite 6** - 极速的开发构建工具
- **lucide-svelte** - 精美的图标库

### 性能指标
- ⚡ **启动时间**: < 1 秒
- 💾 **内存占用**: 30-50 MB
- 📦 **安装包大小**: < 5 MB
- 🔋 **CPU 占用**: 0% (待机状态)

---

## 📖 开发指南

### 环境要求

- **Bun** 或 **Node.js** 18+
- **Rust** 1.82+ ([安装指南](https://rustup.rs/))
- **系统要求**:
  - Windows 10+ (需要 WebView2)
  - macOS 10.13+
  - Linux (需要 WebKit2GTK)

### 本地开发

```bash
# 克隆项目
git clone https://github.com/Kiaana/ClipMan.git
cd ClipMan

# 安装依赖（推荐使用 Bun）
bun install

# 启动开发服务器
bun tauri dev
```

**注意**: macOS 开发模式下可能会显示 Dock 图标，这是正常现象。正式构建版本会隐藏 Dock 图标。

### 构建应用

```bash
# 构建生产版本
bun tauri build

# 构建产物位置:
# Windows: src-tauri/target/release/bundle/msi/
# macOS:   src-tauri/target/release/bundle/dmg/
# Linux:   src-tauri/target/release/bundle/appimage/
```

### 项目结构

```
ClipMan/
├── src/                          # Svelte 前端
│   ├── lib/
│   │   ├── components/           # UI 组件
│   │   │   ├── ClipboardItem.svelte      # 剪切板条目
│   │   │   ├── SearchBar.svelte          # 搜索栏
│   │   │   ├── Toast.svelte              # 提示通知
│   │   │   └── settings/                 # 设置页面组件
│   │   ├── stores/               # 状态管理
│   │   └── utils/                # 工具函数
│   ├── routes/                   # 页面路由
│   └── app.css                   # 全局样式
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs               # 入口、Tauri 配置、系统托盘
│   │   ├── clipboard.rs          # 剪切板监控核心
│   │   ├── storage.rs            # SQLite 数据库（CRUD + FTS5）
│   │   ├── crypto.rs             # AES-256-GCM 加密
│   │   ├── settings.rs           # 设置管理
│   │   └── migration.rs          # 数据迁移
│   ├── Cargo.toml                # Rust 依赖
│   └── tauri.conf.json           # Tauri 配置
└── package.json                  # 前端依赖
```

---

## 🗺️ 开发路线

- [x] 基础剪切板监控和历史记录
- [x] 置顶功能
- [x] 全文搜索
- [x] 全局快捷键
- [x] 系统托盘集成
- [x] 自动更新
- [x] 暗色模式
- [x] AES-256 加密
- [x] 自定义数据存储位置
- [x] 开机自启动
- [x] 智能快捷键录入
- [ ] 剪切板同步（多设备）
- [ ] 分组管理
- [ ] 剪切板规则过滤
- [ ] 插件系统
- [ ] 命令行工具

---

## 🤝 贡献

欢迎任何形式的贡献！请查看 [贡献指南](CONTRIBUTING.md) 了解详情。

### 参与方式
- 🐛 [报告 Bug](https://github.com/Kiaana/ClipMan/issues/new?template=bug_report.md)
- ✨ [提出新功能建议](https://github.com/Kiaana/ClipMan/issues/new?template=feature_request.md)
- 💬 [参与讨论](https://github.com/Kiaana/ClipMan/discussions)
- 📝 提交 Pull Request

### 提交规范
使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：
```
feat: 添加新功能
fix: 修复 Bug
docs: 更新文档
refactor: 代码重构
perf: 性能优化
```

---

## 📄 许可证

本项目采用 [MIT License](LICENSE) 开源协议。

---

## 🙏 致谢

感谢以下优秀的开源项目：

- [Tauri](https://tauri.app/) - 现代桌面应用框架
- [Svelte](https://svelte.dev/) - 超快的前端框架
- [rusqlite](https://github.com/rusqlite/rusqlite) - Rust SQLite 绑定
- [arboard](https://github.com/1Password/arboard) - 跨平台剪切板库
- [ring](https://github.com/briansmith/ring) - 加密库

---

## 📮 联系方式

- **GitHub Issues**: [提交问题](https://github.com/Kiaana/ClipMan/issues)
- **Discussions**: [讨论区](https://github.com/Kiaana/ClipMan/discussions)

---

<div align="center">

**如果 ClipMan 对你有帮助，请给个 ⭐️ Star 支持一下！**

Made with ❤️ by [Kiaana](https://github.com/Kiaana)

</div>
