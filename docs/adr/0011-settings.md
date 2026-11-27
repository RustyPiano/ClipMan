# 0011 设置的所有权、校验与写入

- 状态：已采纳

## 决定

- 有三个字段不由设置页修改：`custom_data_path` 只由 `migrate_data_location` 修改，它会同时搬移数据库；`capture_paused` 只由托盘的“暂停采集”修改；`onboarding_completed` 只由 `complete_onboarding` 修改（ADR 0024）。`update_settings` 总是保留这三个字段的现值，过期的设置窗口或重置请求不会把应用切到空目录，也不会覆盖托盘的暂停状态。
- 托盘开关和 `update_settings` 都持有 `settings_write_lock`，不会互相覆盖。原生菜单回调不直接等这把锁，改在 `spawn_blocking` 里执行，因为持锁的一方可能需要主线程。
- 保存时拒绝超出范围和未知的值。读取旧设置时把数值 clamp 到合法范围，保持兼容。v2.1.x 及更早版本把各项设置直接存在 store 顶层（`LEGACY_SETTINGS_KEYS`），读取时迁移。
- 读取已保存的设置失败时，保留能识别的偏好并暂停采集，不用默认值覆盖出错的文件。启动时自行读取 `settings.json`：文件不存在才按首次启动处理，读取或 JSON 解析失败都按读取失败处理。tauri-plugin-store 2.4.3 首次打开 store 时会忽略这两类错误，返回空 store，不能用它判断。
- `SettingsManager::save` 只写入 store，调用方在保存成功后再用 `set` 更新内存中的设置。
- 数据迁移完成后，设置页只重新读取数据路径，保留用户尚未保存的其他修改。
- 界面语言以后端的 `settings.locale` 为准，各窗口读取设置后调用 `setLocale`。
