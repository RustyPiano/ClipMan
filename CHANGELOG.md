# Changelog

All notable changes to ClipMan will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- GitHub Actions workflow for automated cross-platform releases
- Release documentation and templates

## [1.0.0] - 2025-11-07

### Added
- 🎉 首个正式版本发布
- 剪切板自动监控 (文本和图像)
- SQLite 本地存储with AES-256-GCM 加密
- 全文搜索功能 (内存过滤实现)
- 置顶常用内容
- 清除非置顶历史记录功能
- 系统托盘集成 (显示最近 20 条记录)
- 全局热键支持 (`Cmd+Shift+V` / `Ctrl+Shift+V`)
- 图像缩略图生成 (256x256)
- 设置页面 (热键配置、历史上限)
- macOS 优化 (菜单栏模式、权限检查)
- Svelte 5 现代化 UI

### Fixed
- 搜索功能修复 - 使用内存过滤替代 FTS5 查询
- 窗口同步 - 菜单栏清空历史后自动更新窗口
- Unicode 字符截断问题
- Poisoned lock 恢复机制
- 解密错误处理

### Technical
- Rust 1.82+ 后端
- Tauri 2.0 框架
- Svelte 5 with Runes API
- Tailwind CSS 4
- SQLite + FTS5

## [0.1.0] - 2025-11-06

### Added
- 初始开发版本
- 基础剪切板监控
- 简单历史记录显示
- 基本 UI 界面

---

**说明**:
- `Added` - 新功能
- `Changed` - 现有功能的变更
- `Deprecated` - 即将移除的功能
- `Removed` - 已移除的功能
- `Fixed` - Bug 修复
- `Security` - 安全相关修复
