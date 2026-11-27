## ClipLeaf v2.5.3

### 修复 / Fixes

- 改进数据库和自定义数据目录迁移的文件操作顺序，并支持在中断后继续迁移。
- 设置文件损坏或不可读时暂停剪贴板采集；开机启动更新先登记新项再移除旧项；NSIS 安装遇到旧版卸载失败时中止。
- 隐私标记校验纳入剪贴板序号检查；Windows 无法检查标记时跳过采集；Linux 剪贴板写入复用长期运行的实例。
- 修复切换搜索分类后未重新查询的问题；链接分类改为依据完整内容判断；文件详情直接返回结构化路径列表。

### Fixes

- Improved the file-operation order for database and custom data directory migrations, allowing interrupted migrations to resume.
- Clipboard capture now pauses when the settings file is corrupt or unreadable. Launch-at-login updates register the new entry before removing the old one, and NSIS installation stops if uninstalling the previous version fails.
- Privacy-marker checks now include clipboard sequence validation. Windows skips capture when the marker cannot be checked, and Linux clipboard writes reuse the long-running instance.
- Fixed search results not refreshing when switching categories. Link classification now uses the full content, and file details return a structured path list.
