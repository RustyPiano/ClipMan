# 0007 SQLite 用增量 auto_vacuum 回收空间

- 状态：已采纳

## 决定

- 数据库使用 `auto_vacuum = INCREMENTAL`。这个设置要在 `VACUUM` 之后才生效，旧数据库在 `ensure_incremental_auto_vacuum` 里完整重写一次，之后打开时直接跳过。
- 删除和裁剪后调用 `reclaim_space`。SQLite 不允许在事务里运行 `incremental_vacuum`，所以调用方要在自己的事务提交后再调用。回收失败只记日志，不把已经成功的删除变成错误。
- 每个连接打开时设置 `secure_delete = ON`，删除的内容用零覆盖，不留在空闲页里。FTS5 索引另有自己的 `secure-delete` 选项，`initialize_fts` 每次打开都写入，已有数据库也会开启；否则删除的文本片段会留在 `clips_fts_data` 的索引块里。清空历史时在同一事务里执行 FTS5 `optimize`，清掉开启这个选项之前删除留下的片段。
- 清空历史后执行 `wal_checkpoint(TRUNCATE)`，WAL 文件里也不保留旧页面。其他程序持有读事务时检查点通过结果行的 busy 列报告失败，SQLite 不返回错误；这时历史已经删除，命令返回警告，提示关闭占用数据库的程序后再清空一次。
- 升级前备份和损坏隔离文件不会自动删除，用户在“存储”页查看并删除。只识别 ClipLeaf 和 ClipMan 实际生成的完整文件名：`{clipleaf,clipman}.pre-v3-<UUID>.db` 和 `{clipleaf,clipman}.db.corrupt-<UUID>`（含 `-wal`、`-shm`、`-journal`），自定义数据目录里的其他文件不会被删除。
- `PRAGMA incremental_vacuum` 每释放一批页返回一行，必须作为查询逐行读完。用 `execute()` 会在第一页后报 "Execute returned results"，其余空闲页不会被回收。
