# ClipLeaf

<div align="center">

<img src="website/public/icon.png" alt="ClipLeaf" width="128" />

**轻量级现代剪贴板管理器**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/github/v/release/RustyPiano/ClipLeaf?label=Version&color=blue)](https://github.com/RustyPiano/ClipLeaf/releases/latest)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/RustyPiano/ClipLeaf)

[下载使用](#安装) · [功能特性](#功能) · [开发文档](#开发)

[English](README_EN.md)

</div>

## 简介

ClipLeaf（原名 ClipMan）是使用 Rust 和 Svelte 5 开发的桌面剪贴板管理器，历史记录保存在本地 SQLite 数据库。

## 功能

### 核心特性

- **收藏常用内容** - 一键收藏代码片段、命令、链接或直接新建常用文本与提示词，支持排序与预览栏在线编辑
- **持久化存储** - SQLite 本地数据库，重启不丢失，智能去重
- **全文搜索与分类筛选** - 搜索历史记录时会高亮关键词。切换到文本、图片、文件或链接分类后，会在该分类下重新搜索
- **多类型预览** - 统一保存纯文本、HTML 富文本、图片和文件列表；预览栏支持图片清晰展示、色值 HEX/RGB/HSL 一键复制、链接直开与在访达/文件夹中显示，macOS 显示来源应用图标
- **QuickBar 快捷取用** - 任意应用快速调出（默认 `Cmd/Ctrl+Shift+V`，再次按下可关闭），支持原生中文输入法，键盘选择并按设置自动粘贴或仅复制
- **粘贴格式** - 保留原始格式、仅经 ClipLeaf 取用时去除格式，或复制富文本后直接改为纯文本；`⌥Enter` 临时切换一次
- **键鼠操作** - 每行可直接复制、收藏、编辑标签、删除；`⇧↑/↓`、`⇧`+点击选择连续范围，`⌘`+点击单独勾选，支持批量收藏、批量删除与回车合并粘贴
- **托盘菜单** - 快速打开 QuickBar 或复制最近和收藏内容
- **多主题** - 浅色/深色/淡粉色主题，跟随系统
- **多语言** - 支持中文和英文，自动检测系统语言

### 其他功能

- 可跳过私钥和常见 API 令牌。默认跳过密码管理器等标记为隐私的内容（macOS、Windows 和 Linux X11），无法检查标记时跳过该条记录
- 首次启动引导：快捷键、辅助功能授权和使用说明
- 后台自动检查更新，显示下载进度，装完由你决定何时重启
- 设置修改后自动保存
- 开机自启动
- 自定义存储位置，查看占用空间、导出/导入收藏、删除旧备份
- 删除的记录会从数据库和索引中覆盖清除
- 日志和诊断信息（不含剪贴板内容），方便反馈问题

## 安装

从[官网](https://clipleaf.app)选择安装包，或访问 [GitHub Releases](https://github.com/RustyPiano/ClipLeaf/releases/latest) 下载：

- **Windows**: `ClipLeaf_2.6.0_x64_en-US.msi`
- **macOS (Apple Silicon)**: `ClipLeaf_2.6.0_aarch64.dmg`
- **macOS (Intel)**: `ClipLeaf_2.6.0_x64.dmg`
- **Linux**: `ClipLeaf_2.6.0_amd64.AppImage`

从 ClipMan 升级时，ClipLeaf 会迁移历史记录、收藏和设置。如果数据库改名时中断，下次启动会继续。

### macOS 安装与权限

**首次打开**：当前版本以自签名证书签名、但未做 Apple 公证，首次启动可能被 Gatekeeper 拦截（“无法打开，因为无法验证开发者”）。解决办法：在 **访达** 中右键点击 ClipLeaf → **打开**，或前往 **系统设置** → **隐私与安全性**，在底部点击 **仍要打开**。之后即可正常启动。

**辅助功能权限**：自动粘贴（模拟 Cmd+V）需要辅助功能权限。首次运行按提示授予，或手动添加：

**系统设置** → **隐私与安全性** → **辅助功能** → 添加并勾选 ClipLeaf

> 若某次更新后自动粘贴失效，通常是该权限失效了。ClipLeaf 会弹窗引导你重新授权——在上面的列表里把 ClipLeaf 重新勾选一次即可（其间复制照常，内容仍会进入剪贴板，可手动粘贴）。

## 使用

1. 使用快捷键 `Cmd+Shift+V` (macOS) / `Ctrl+Shift+V` (Windows) 打开
2. 复制文本、图片或文件后，在历史列表中查看记录
3. 点击星标图标收藏常用内容，或在「收藏」面板直接新建片段
4. 搜索框快速查找历史记录
5. 点击条目执行 QuickBar 默认取用行为：默认自动粘回当前应用；关闭自动粘贴后仅复制
6. 点击托盘图标快速访问（托盘菜单始终仅复制）

## 技术

**后端**

- Tauri 2.11 - 轻量桌面框架
- SQLite + FTS5 - 高性能数据库
- Rust - 内存安全，性能卓越

**前端**

- Svelte 5 - 现代响应式框架
- Tailwind CSS 4 - 现代样式方案
- Vite 8 - 快速构建工具

**性能定位**

- 轻量弹窗：QuickBar 使用隐藏小窗口，按需唤起
- 本地优先：SQLite 持久化，无后台网络服务
- 索引搜索：FTS5 + trigram 索引用于降低搜索延迟

## 开发

### 环境要求

- Bun 1.3+
- Rust 1.96.0（项目通过 `rust-toolchain.toml` 固定）
- 系统: Windows 10+（WebView2 111+） / macOS 13.3+ / Linux（WebKitGTK 4.1，支持现代 CSS）

### 快速开始

```bash
# 克隆项目
git clone https://github.com/RustyPiano/ClipLeaf.git
cd ClipLeaf

# 安装依赖
bun install

# 开发
bun tauri dev

# 构建
bun tauri build
```

开发规范与设计决策见 [AGENTS.md](AGENTS.md) 和 [docs/adr/](docs/adr/)。

## 路线图

- [ ] 智能图片压缩（AVIF/WebP/MozJPEG，可配置）
- [ ] 规则过滤
- [ ] 插件系统
- [ ] 命令行工具

## 贡献

欢迎通过 Issue 或 Pull Request 贡献。

- [报告 Bug](https://github.com/RustyPiano/ClipLeaf/issues)
- [功能建议](https://github.com/RustyPiano/ClipLeaf/issues)
- [讨论区](https://github.com/RustyPiano/ClipLeaf/discussions)

## 许可证

[MIT License](LICENSE)

## 致谢

- [Tauri](https://tauri.app/) - 桌面应用框架
- [Svelte](https://svelte.dev/) - 前端框架
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite 绑定
- [arboard](https://github.com/1Password/arboard) - 剪贴板库

---

<div align="center">

**如果觉得有帮助，请给个 Star！**

Made by [RustyPiano](https://github.com/RustyPiano)

</div>
