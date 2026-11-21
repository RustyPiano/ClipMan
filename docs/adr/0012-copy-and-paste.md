# 0012 复制与粘贴流程

- 状态：已采纳

## 决定

- 复制、单条粘贴和合并粘贴共用 `paste::use_clips`。写入剪贴板成功后才更新使用记录；写入之后的步骤失败时不报告为可以重试的粘贴失败，避免用户再粘贴一次。
- macOS 自动粘贴前先检查 `AXIsProcessTrusted()`。缺少辅助功能权限时，enigo 发送 Cmd+V 用的 `CGEventPost` 会静默失败并照常返回 Ok，无法从粘贴结果发现问题。权限缺失时发出 `accessibility-permission-required` 事件并引导用户重新授权；内容已经在剪贴板上，用户可以手动粘贴。系统按代码签名保存授权，应用更新后常会失效。
- 模拟粘贴会阻塞：先经主线程把原来的前台应用调回前台（最多等 5 秒），再发送 Cmd+V。这一步在 blocking 线程池里运行，不占用 Tokio worker。
- 发送前先恢复打开 QuickBar 时记录的前台应用；如果 QuickBar 在发送前又被打开，就放弃发送，Cmd+V 不会发给 QuickBar 自己。
- macOS 上用 V 键的虚拟键码 `kVK_ANSI_V = 9`，不用 `Key::Unicode('v')`。enigo 解析字符时会调用 `TSMGetInputSourceProperty`，这个接口要求在主队列上运行，在 Tokio worker 上调用会让整个进程以 EXC_BREAKPOINT 退出。

## 已知问题

恢复前台应用后固定等待 60 ms 再发送，见 #7。
