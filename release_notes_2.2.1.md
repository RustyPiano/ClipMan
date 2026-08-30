## ClipMan v2.2.1

### 修复

- **更安全的数据库恢复。** 仅在 SQLite 明确认定数据库损坏时重建历史；其他 I/O 或权限错误不会移动原数据库，损坏数据库及其关联文件会作为一组可恢复备份隔离。
- **更安全的数据目录迁移。** 不再覆盖目标目录中已有的 ClipMan 数据库；删除旧数据改为主动勾选，清理或监控重启异常会明确提示，迁移也不会丢弃其他尚未保存的设置。
- **快捷键回滚更可靠。** 某条快捷键更新失败时，不再误注销另一条未修改的快捷键。
- **消除启动期间的历史显示空窗。** ClipMan 会先监听剪贴板事件再加载初始历史，启动过程中新增的条目也能可靠显示。
- **错误恢复更清晰。** 历史与搜索失败会显示原因并支持重试；保存后的设置与后端校验结果保持一致，迁移弹窗也改善了键盘与屏幕阅读器操作。

### 其他

- 扩展 Linux / Windows CI、增加前端测试类型检查，并在打包前重跑全部质量门。
- 统一使用 Bun 锁文件管理依赖。

---

### Fixes

- **Safer database recovery.** ClipMan now resets history only when SQLite explicitly reports database corruption. Other I/O or permission failures leave the original database untouched, and corrupt database files are quarantined as one recoverable set.
- **Safer data-location migration.** Existing ClipMan databases are no longer overwritten. Removing old data is now opt-in, cleanup or monitor-restart warnings are surfaced, and migration no longer discards unrelated unsaved settings.
- **Reliable shortcut rollback.** A failed shortcut update no longer unregisters the shortcut that was not changed.
- **No startup history blind spot.** Clipboard events are subscribed before the initial history query, so newly captured items appear reliably while ClipMan starts.
- **Clearer recovery UI.** History and search failures are visible and retryable; saved setting values now reflect backend validation, and the migration dialog has improved keyboard and screen-reader behavior.

### Other

- Expanded Linux and Windows CI coverage, type-checks frontend tests, and reruns all quality gates before packaging.
- Bun is now the single dependency-lock workflow.
