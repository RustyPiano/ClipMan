## ClipLeaf v2.6.0

### 新功能

- 首次启动显示欢迎页，介绍快捷键、辅助功能授权（macOS）和使用方法。在系统设置里授权后，欢迎页会自动回来。历史为空时显示操作提示。
- 后台每天检查一次更新，可在“关于”页关闭。有新版本时托盘菜单和 QuickBar 底栏会提示，安装时显示下载进度，装完由你决定何时重启。
- 设置改为修改后自动保存。“托盘”页并入“常规”页；“存储”页显示占用空间，支持导出和导入收藏、删除旧备份文件。调低历史记录数量会删除旧记录时，先确认要删除的条数。
- 日志写入系统日志目录，程序崩溃时会记录原因。“关于”页可以复制诊断信息、打开日志目录，两者都不包含剪贴板内容。

### 隐私与修复

- 删除的记录会从数据库和全文索引中覆盖清除，清空历史后同时清理数据库日志文件。
- Linux（X11）识别密码管理器设置的密码标记，开启“忽略密码类剪贴板”后不再保存 KeePassXC 等复制的密码；连续复制时确保标记和内容来自同一次复制。
- 密钥检测新增 Stripe、Google、GitLab、npm 令牌、PGP 私钥和带密码的数据库连接串。
- macOS 和 Linux 检查剪贴板的间隔从 500 毫秒缩短到 100 毫秒，快速连续复制时更少遗漏。
- 权限提示同时适用于从未授权和更新后失效两种情况。

### New

- A welcome page on first launch explains the shortcut, Accessibility access (macOS), and how to use ClipLeaf. It comes back on its own after you grant access in System Settings. An empty history now shows a hint.
- ClipLeaf checks for updates once a day in the background (you can turn this off in About). New versions are announced in the tray menu and the QuickBar footer, installation shows download progress, and you choose when to restart.
- Settings now save automatically. Tray options moved into General. The Storage page shows disk usage, exports and imports favorites, and removes old backup files. Lowering the history limit asks before deleting older items.
- Logs are written to the system log folder, including crash details. About can copy diagnostics and open the log folder; neither includes clipboard contents.

### Privacy and fixes

- Deleted items are overwritten in the database and the full-text index, and clearing history also cleans up the database journal.
- On Linux (X11), ClipLeaf recognizes the password marker set by password managers, so passwords copied from KeePassXC and similar apps are skipped when "Ignore password-type clipboard" is on. Rapid copies can no longer pair one copy's marker with another copy's content.
- Secret detection now covers Stripe, Google, GitLab, and npm tokens, PGP private keys, and database connection strings with passwords.
- macOS and Linux check the clipboard every 100 ms instead of 500 ms, so fast consecutive copies are missed less often.
- The permission prompt now fits both a first-time grant and access lost after an update.
