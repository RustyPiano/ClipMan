## ClipMan v2.5.0

### 新功能与体验改进

- **macOS 中文输入法修复与快捷键切换**：改用标准可激活浮动窗口，修复 QuickBar 搜索框内中文/CJK 输入法候选窗不显示的问题（兼容 macOS 13.3+）。同一面板重复按全局快捷键可关闭并恢复原前台应用，按另一面板快捷键直接切换面板；自动粘贴前轮询确认原应用回到前台，复制模式隐藏和 `Esc` 隐藏也会恢复原应用焦点。
- **收藏体验重构（原“置顶”更名为“收藏”）**：统一采用星标（`★`）标识与即时反馈，顶部显示收藏计数；收藏面板支持直接新建常用文本/提示词、鼠标上下排序和批量收藏/批量删除；预览栏支持直接编辑正文（同事务同步全文索引，超 1 MiB 截断项禁止编辑，窗口隐藏不丢失草稿）。
- **QuickBar 视觉与预览栏升级**：重做浅色、深色、淡粉色三套配色与界面布局；新增全库 SQL 分类筛选（全部 / 文本 / 图片 / 文件 / 链接）与搜索关键词高亮；预览栏支持普通图片自动加载清晰原图、色值卡片 HEX/RGB/HSL 一键复制、链接直开以及在访达/文件夹中显示。
- **设置页与托盘改进**：设置页新增未保存状态提示和 `⌘S` 保存；托盘菜单顶部新增“打开 ClipMan”。

### 修复

- 外部链接打开与文件定位统一改用 `tauri-plugin-opener`，修复 Windows 路径含空格及特殊字符 URL 的处理问题。
- 修复批量收藏打乱已有收藏顺序、触控板微抖误切选中行、以及长文本预览截断边界等问题。

### What's New & Improvements

- **macOS CJK IME fix & shortcut toggle**: Switched QuickBar to a standard activatable floating window (macOS 13.3+ compatible) so Chinese/CJK IME candidate windows render reliably in the search box. Pressing the active panel's global shortcut toggles QuickBar closed and restores the previous app; pressing the other panel's shortcut switches panels directly. Auto-paste now waits for the target app to become frontmost, and copy-only or `Esc` hides also restore focus.
- **Favorites overhaul (renamed from Pinned)**: Replaced pin badges with a persistent star (`★`), instant feedback, and a favorites counter; create custom text/prompt favorites directly, reorder them with mouse controls, batch favorite/delete, and edit text inline in the preview pane (with FTS sync, 1 MiB truncation guard, and draft preservation across hides).
- **QuickBar UI & inspector preview**: Rebuilt Light, Dark, and Pink themes and layout; added SQL-backed category filters (All / Text / Images / Files / Links) and search match highlighting; inspector preview now auto-loads images cleanly, previews colors with one-click HEX/RGB/HSL copy, opens URLs, and reveals files in Finder/Explorer.
- **Settings & tray**: Added unsaved indicator and `⌘S` save in Settings, plus "Open ClipMan" at the top of the tray menu.

### Fixes

- Switched URL opening and folder reveal to `tauri-plugin-opener`, fixing Windows command escaping and paths with spaces.
- Preserved existing favorite ordering during batch favorite, fixed trackpad hover jitter after keyboard navigation, and ensured UTF-8 safe preview truncation.
