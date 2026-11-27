## ClipLeaf v2.5.2

### 完成更名

- 应用的内部标识、数据库文件和 macOS 签名证书全部改用 ClipLeaf。更新后首次启动会自动把原有的历史记录、收藏和设置迁移过来，无需任何操作。
- macOS：“应用程序”中的 `ClipMan.app` 会自动改名为 `ClipLeaf.app`。
- Windows 和 Linux：自动替换旧安装，开机自启随之迁移。

### 需要你操作一次（macOS）

- 由于应用标识和签名已更换，**自动粘贴需要重新授权一次**：打开“系统设置 → 隐私与安全性 → 辅助功能”，勾选 ClipLeaf（列表里旧的 ClipMan 可以删除）。未授权前选中内容仍会复制到剪贴板，可手动粘贴。

### Renaming completed

- The app identifier, database file, and macOS signing certificate now use ClipLeaf. Your history, favorites, and settings are migrated automatically on the first launch after updating.
- macOS: `ClipMan.app` in Applications is renamed to `ClipLeaf.app` automatically.
- Windows and Linux: the update replaces the old installation and carries over launch-at-login.

### One-time action on macOS

- Because the app identifier and signature changed, **auto-paste must be re-enabled once**: open System Settings → Privacy & Security → Accessibility and enable ClipLeaf (you can remove the old ClipMan entry). Until then, selected items are still copied to the clipboard so you can paste manually.
