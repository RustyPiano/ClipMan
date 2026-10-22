## ClipMan v2.4.5

### 修复与改进

- 修复中文输入法确认输入时，回车误触发粘贴的问题。
- 文件列表展示前两个文件，剩余数量使用实际文件总数；重复文件路径可以正常显示。
- 保存设置时检查数值范围、语言和粘贴格式，超出范围时提示错误；保留旧设置的读取兼容。
- 存储目录读取失败后可以重新检查，打开目录失败时显示错误。
- 合并重复的纯文本写入代码，精简无效判断、重复注释和文档，保留容量、并发及写入检查。

### Fixes and improvements

- Prevent Enter from pasting a clipboard item when confirming Chinese input.
- Show the first two files and calculate the remaining count from the full file list. Duplicate paths render correctly.
- Validate settings ranges, language and paste format when saving, while retaining compatibility when loading older settings.
- Allow retrying a failed data-directory lookup and show errors when opening the directory fails.
- Share plain-text clipboard writing and simplify redundant checks, comments and documentation while preserving size limits, concurrency controls and write verification.
