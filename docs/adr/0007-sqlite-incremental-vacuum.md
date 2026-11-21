# 0007 SQLite 用增量 auto_vacuum 回收空间

- 状态：已采纳

## 决定

- 数据库使用 `auto_vacuum = INCREMENTAL`。这个设置要在 `VACUUM` 之后才生效，旧数据库在 `ensure_incremental_auto_vacuum` 里完整重写一次，之后打开时直接跳过。
- 删除和裁剪后调用 `reclaim_space`。SQLite 不允许在事务里运行 `incremental_vacuum`，所以调用方要在自己的事务提交后再调用。回收失败只记日志，不把已经成功的删除变成错误。
- `PRAGMA incremental_vacuum` 每释放一批页返回一行，必须作为查询逐行读完。用 `execute()` 会在第一页后报 "Execute returned results"，其余空闲页不会被回收。
