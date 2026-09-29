## ClipMan v2.4.0

### 新功能与体验改进

- **粘贴格式三种模式。** 设置 → 粘贴格式：「保留原始格式」按复制时的原样粘贴；「取用时纯文本」只在经 ClipMan 取用或复制时去除格式；「始终纯文本」在复制富文本后立即把系统剪贴板改为纯文本，在 Word/PPT 里直接 Cmd+V 也不带格式。`⌥Enter` 可以临时切换一次。
- **QuickBar 行恢复信息密度。** 每行显示标签、两行内容预览（文件显示文件名和所在目录，图片显示缩略图）和时间、来源等元信息，操作按钮不遮挡文字。

### 修复与可靠性

- 文件写回剪贴板失败时直接报错，不再改为粘贴路径文本。
- 自定义数据目录不可用或剪贴板监听无法启动时，弹出错误后退出，不再悄悄改用默认目录。
- 在 WebKit 中修复图片缩略图被拉满整行、元信息多出空格、多选标记遮挡文字等显示问题；界面统一使用「剪贴板」。
- 更新说明改为原文显示。

### 其他

- 发布流程增加版本预检查、更新公钥比对，并在公开前下载全部附件校验更新签名。
- 移除不再使用的代码、依赖和生成图标，应用权限只保留实际用到的部分。

---

### New and improved

- **Three paste formats.** Settings → Paste format: *Keep original formatting* pastes exactly as copied; *Plain text via ClipMan* strips formatting only when taking or copying through ClipMan; *Always plain text* turns copied rich text into plain text on the system clipboard right away, so a direct Cmd+V into Word/PPT lands unformatted. `⌥Enter` flips the mode for one paste.
- **Denser QuickBar rows.** Each row shows its label, a two-line preview (file names with their folder, or an image thumbnail) and time/source metadata, with actions that never cover the text.

### Fixes and reliability

- A failed file write-back now reports an error instead of pasting the paths as text.
- An unusable custom data directory or a clipboard monitor that cannot start now shows an error and exits instead of silently switching to the default directory.
- Fixed WebKit display issues: stretched image thumbnails, extra spaces in row metadata, and the multi-select badge covering text.
- Update notes are shown as plain text.

### Other

- The release pipeline adds version prechecks, an updater public-key comparison, and signature verification of every downloaded asset before publishing.
- Removed unused code, dependencies and generated icons; app capabilities now cover only what is used.
