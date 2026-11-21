# 0008 数据库格式升级

- 状态：已采纳

## 决定

- `migration::upgrade_clip_database_to_current` 返回是否需要重建 FTS 索引。调用方在索引重建成功后才写入新的 `user_version`，中途失败时下次打开会重新升级。
- 格式 v3 把文件列表存为 JSON 字符串数组。2.2.x 不拒绝 v3 数据库，降级期间可能写入换行分隔的旧格式，所以每次打开数据库都运行 `convert_legacy_file_lists`，即使版本号已经是 3。
- 升级前先用 SQLite backup 写一份带 UUID 文件名的备份。`backup_connection` 要求目标文件不存在（迁移时 `prepare_destination_directory` 已拒绝含数据库文件的目录），所以备份失败时可以直接删除本次创建的文件。
