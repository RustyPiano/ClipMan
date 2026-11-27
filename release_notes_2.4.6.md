## ClipMan v2.4.6

### 修复与改进

- macOS：打开或点击 QuickBar 时保持目标应用处于前台，避免中断访达改名等正在进行的操作。
- 数据迁移补建缩略图时保留小图原尺寸，修复托盘菜单多余的分隔线和设置页重复的托盘提示。
- 精简剪贴板处理、数据库索引维护和快捷键更新代码。Windows 改用现有的系统接口绑定读取系统语言和处理剪贴板隐私标记。

### Fixes and improvements

- macOS: Keep the target app active when opening or clicking QuickBar so actions such as renaming a file in Finder can continue.
- Preserve the original dimensions of small images when backfilling thumbnails during migration. Remove extra tray menu separators and duplicate tray guidance in Settings.
- Simplify clipboard handling, database index maintenance, and shortcut updates. On Windows, use existing system bindings for language detection and clipboard privacy markers.
