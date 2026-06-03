# ClipMan v1.10.0

## ✨ New Features
- QuickBar: open clipboard history with `Cmd/Ctrl+Shift+V`, search, select by keyboard, and use the selected item
- Auto-paste mode for QuickBar, with copy-only fallback when auto-paste is disabled
- SQLite FTS5 + trigram search for faster Chinese/English text search
- Separate recent and pinned lists, with pinned labels and stable ordering support
- Skip password-like / concealed clipboard contents when supported by the platform
- Image capture now stores original content plus a derived thumbnail for faster list rendering
- Internationalization: English & Chinese UI support with auto language detection

## 🚀 Improvements
- Main window is now a lightweight QuickBar-style popup with a separate settings window
- Keyboard navigation, search feedback, and highlighted selection were optimized for a smoother native feel
- Copy from window/tray now moves item to top (same as duplicate copy behavior)

## 🧹 Maintenance
- Removed the legacy local AES layer so storage, migration, and indexed search use a simpler local-first data model

---

# ClipMan v1.10.0 更新日志

## ✨ 新功能
- QuickBar：通过 `Cmd/Ctrl+Shift+V` 打开剪切板历史，支持搜索、键盘选择和取用
- QuickBar 自动粘贴模式；关闭自动粘贴后退化为仅复制
- SQLite FTS5 + trigram 搜索，提升中英文文本搜索速度
- 最近与置顶列表拆分，支持置顶标签和稳定排序
- 支持跳过平台标记的密码类 / 隐藏剪切板内容
- 图片捕获改为保存原图并派生缩略图，列表渲染更轻量
- 国际化：支持中文和英文界面，自动检测系统语言

## 🚀 优化
- 主窗口改为轻量 QuickBar 弹窗，并拆出独立设置窗口
- 优化键盘导航、搜索反馈和选中高亮，降低视觉卡顿
- 从窗口/托盘复制时，内容自动置顶（与复制重复内容行为一致）

## 🧹 维护
- 移除旧的本地 AES 层，让存储、迁移和索引搜索回到更简单的本地优先数据模型
