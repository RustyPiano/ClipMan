# ClipMan v2.2.0

## New
- **Files & folders.** Copy files in Finder and they're saved to your history like any other clip — paste them back into any app later.
- **Rich text.** Formatted text (from browsers, docs, editors) keeps its formatting when captured and pasted. Prefer plain? Press **⌥Enter** to paste any clip as plain text.
- **Multi-select & merge-paste.** **⌘-click** to select several clips, then paste them all at once as one combined block.
- **Skip secrets.** ClipMan can detect and skip password-manager entries and things that look like API keys/tokens, so they never land in your history.
- **Ignore apps & pause capture.** Exclude specific apps from being captured, or pause capture entirely from the tray when you need a break.

## Improvements
- **Smoother, endless history.** The list now loads as you scroll instead of stopping at a fixed limit.
- **Cleaner dark mode.** Fixed a stray window shadow/halo around the panel in dark mode.
- **Leaner build.** Dropped unused components and trimmed dependencies for a smaller, faster app.
- **More robust startup & storage.** Recovers gracefully if data can't be loaded, and reclaims disk space more reliably over time.

## Fixes
- **Pasting files now works reliably on macOS.** Recent macOS silently dropped file paste when ClipMan lacked disk access. ClipMan now requests access at the right moment, verifies the paste landed, and falls back to text (with a heads-up) if the system blocks it. For the smoothest experience, grant ClipMan **Full Disk Access**.

## Upgrading
- The first launch may show a Gatekeeper "unverified developer" prompt — right-click the app and choose **Open**, or allow it under **System Settings → Privacy & Security**.
- For file copy/paste, grant **System Settings → Privacy & Security → Full Disk Access → ClipMan** (or approve the per-folder prompt on first paste).

---

# ClipMan v2.2.0 更新日志

## 新功能
- **文件与文件夹。** 在访达里复制文件即可像普通条目一样存入历史，之后可粘贴回任意应用。
- **富文本。** 来自浏览器、文档、编辑器的带格式文本，采集与粘贴时会保留格式。想要纯文本？按 **⌥Enter** 即可将任意条目以纯文本粘贴。
- **多选合并粘贴。** **⌘ 点击**可多选若干条目，一次性合并为一整块粘贴。
- **跳过秘密内容。** ClipMan 能识别并跳过密码管理器条目、以及形似 API 密钥/令牌的内容，让它们不进入历史。
- **忽略应用与暂停采集。** 可将指定应用排除在采集之外，或从托盘一键暂停采集。

## 改进
- **更顺滑的无限历史。** 列表随滚动持续加载，不再停在固定条数。
- **更干净的深色模式。** 修复了深色模式下面板周围的窗口阴影/灰边。
- **更精简的构建。** 移除未使用的组件、裁剪依赖，应用更小更快。
- **更稳健的启动与存储。** 数据无法加载时也能优雅降级恢复，并更可靠地回收磁盘空间。

## 修复
- **macOS 上粘贴文件现已稳定可用。** 较新的 macOS 会在 ClipMan 缺少磁盘访问权限时静默丢弃文件粘贴。现在 ClipMan 会在正确时机申请权限、校验粘贴是否落地，并在系统拦截时降级为文本（并给出提示）。为获得最佳体验，请授予 ClipMan **完全磁盘访问权限**。

## 升级说明
- 首次打开可能出现 Gatekeeper“无法验证开发者”的提示——右键点击应用选择**打开**，或在 **系统设置 → 隐私与安全性** 中允许。
- 文件复制/粘贴需授予 **系统设置 → 隐私与安全性 → 完全磁盘访问权限 → ClipMan**（或在首次粘贴时同意按目录的授权弹窗）。
