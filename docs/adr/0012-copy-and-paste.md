# 0012 复制与粘贴流程

- 状态：已采纳

## 决定

- 复制、单条粘贴和合并粘贴共用 `paste::use_clips`；独立纯文本复制（如色值 HEX/RGB/HSL）走 `copy_plain_text`，共用同一把 `clipboard_use_lock` 与 `last_copied_by_us` 自身复制标记。写入剪贴板成功后才更新使用记录；写入之后的步骤失败时不报告为可以重试的粘贴失败，避免用户重复粘贴。
- macOS 自动粘贴前先检查 `AXIsProcessTrusted()`。缺少辅助功能权限时，`enigo` 发送 Cmd+V 用的 `CGEventPost` 会静默失败并照常返回 Ok。权限缺失时发出 `accessibility-permission-required` 事件并引导用户重新授权；内容已写入剪贴板，用户可手动粘贴。系统按代码签名保存授权，应用更新后若签名变化会失效。
- 模拟粘贴在 blocking 线程池运行：先经主线程把打开 QuickBar 前记录的原前台应用调回前台，再每 10 ms 轮询 `NSWorkspace.frontmostApplication` 确认其进程 PID 已成为前台（上限 500 ms，再留 20 ms 稳定），超时则降级返回 `copiedOnly` 而非盲发 Cmd+V；若发送前 QuickBar 重新打开也放弃发送。
- 复制模式隐藏（关闭自动粘贴按回车、或开启时按 `⌘↵`）以及已授权但粘贴失败时，同样经主线程调用 `hide_quickbar_and_restore` 恢复原前台应用，避免 ClipMan 停留在前台导致用户手动按 `⌘V` 无效。
- macOS 上用 V 键的虚拟键码 `kVK_ANSI_V = 9`，不用 `Key::Unicode('v')`：`enigo` 解析字符时会调用要求主队列执行的 `TSMGetInputSourceProperty`，在后台线程调用会触发 `EXC_BREAKPOINT`。
- 外部链接打开与文件定位统一通过 `tauri-plugin-opener` 执行，不直接调用 `cmd /C start` 或未回收子进程的 `Command::spawn()`。
