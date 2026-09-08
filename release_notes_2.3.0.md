## ClipMan v2.3.0

### 新功能与体验改进

- **重新设计 QuickBar。** 统一浅色、深色和淡粉色视觉；列表保持紧凑，同时恢复每行复制、置顶、标签、删除和多选操作，不再挤占正文宽度。
- **更快、更稳定的历史浏览。** 固定行高虚拟列表可流畅处理大量记录；搜索覆盖完整内容、显示命中摘要，并修复快速输入、立即回车及异步刷新竞态。
- **完整内容按需预览。** 文本、图片和文件摘要保持轻量，完整文本与原图仅在需要时读取；多选内容可按顺序合并取用。

### 修复与可靠性

- 修复设置保存与托盘暂停可能互相等待、设置窗口隐藏后快捷键未恢复，以及异步图片捕获顺序错误。
- 历史上限现在覆盖重复复制、降低上限和取消置顶；文件列表使用无损格式保存，支持文件名中的换行并兼容旧记录。
- 文件粘贴会验证全部 URL 是否真正写入；失败时明确降级为路径文本。合并取用增加 50 MB 总量保护。
- 数据目录显示、迁移和清理统一使用实际打开的数据库；设置读取失败会暂停采集并保留原文件。
- 数据库升级到 v3，升级前自动备份。相同 1 MiB 备份探针由约 1.52 秒降至约 1.74 毫秒。

### 其他

- 发布门禁新增 Chromium 与 WebKit 交互回归；依赖已更新并通过完整漏洞审计。
- 最低环境调整为 macOS 13.3+；Windows 需要 WebView2 111+。
- 完整回归通过：132 项 Rust、56 项前端单元测试、36 项双浏览器交互测试，以及 8 轮 macOS 原生自动粘贴。

---

### New and improved

- **Redesigned QuickBar.** Light, dark, and pink themes now share one visual system. Per-row copy, pin, label, delete, and multi-select actions are back without reducing the text width.
- **Faster, more reliable history.** A fixed-height virtual list handles large histories efficiently. Search covers full content, shows match excerpts, and fixes fast-input, immediate-Enter, and async refresh races.
- **Details on demand.** Text, image, and file summaries stay lightweight while full text and original images load only when requested. Selected clips can be merged in order.

### Fixes and reliability

- Fixed a possible settings/tray deadlock, shortcuts remaining disabled after hiding Settings, and delayed image captures receiving the wrong order.
- History limits now apply to duplicate captures, reduced limits, and unpinning. File lists use a lossless format that supports newlines in filenames while remaining compatible with old records.
- File paste verifies that every URL reached the clipboard and clearly falls back to path text on failure. Merge paste now has a 50 MB aggregate limit.
- Data-path display, migration, and cleanup use the database actually opened. A settings read failure pauses capture and preserves the original file.
- Database format v3 creates a backup before upgrading. The same 1 MiB backup probe improved from about 1.52 seconds to about 1.74 milliseconds.

### Other

- Release gates now include Chromium and WebKit interaction tests. Dependencies were updated and pass a full vulnerability audit.
- Minimum requirements are now macOS 13.3+ and WebView2 111+ on Windows.
- Full regression passed: 132 Rust tests, 56 frontend unit tests, 36 browser interaction tests, and 8 native macOS auto-paste rounds.
