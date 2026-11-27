# 0014 macOS QuickBar 窗口激活与中文输入法

- 状态：已采纳

## 决定

- macOS 上 QuickBar 使用标准浮动窗口（`NSFloatingWindowLevel` + `Auxiliary | CanJoinAllSpaces | FullScreenAuxiliary | Transient | IgnoresCycle`），打开时通过 `window.set_focus()`（tao 内部调用 `makeKeyAndOrderFront` 与 `activateIgnoringOtherApps:YES`）激活 Accessory 应用，不使用 macOS 14.0+ 专属的 `NSApp.activate()` 以保持 macOS 13.3 兼容。`Auxiliary`（macOS 13.0+）不可省略：缺少它时 ClipLeaf 会被激活，但窗口不会出现在其他应用的全屏 Space 上。
- 不使用 `object_setClass` 运行时替换 `NSPanel` 或私有接口 `_setPreventsActivation:`：在 `WKWebView`（跨进程 WebContent）下阻止应用激活会导致 macOS 输入法服务无法挂载中/日/韩候选窗，并引入私有 API 风险。
- 打开前记录原前台应用 PID。粘贴前轮询 `NSWorkspace.frontmostApplication`（上限 500 ms）确认原应用回到前台后再发送 `⌘V`，超时降级为 `copiedOnly`。复制模式隐藏或按 `Esc` / 再次按下同一面板全局快捷键隐藏（且设置窗口未显示）时同样恢复原应用焦点。

## 取舍

- 打开 QuickBar 时原前台应用会短暂失焦。常规输入框（编辑器、浏览器、终端、聊天软件等）在恢复焦点后光标保持原位，粘贴不受影响。
- 少数在失焦时立即退出编辑态的临时控件（最典型是 Finder 行内重命名文件名）会退出编辑态，后续自动粘贴会在目录中生成 `.textClipping`。在 Tauri + `WKWebView` 架构下，选择优先保证搜索框的原生中文输入法可用，并接受上述边缘场景限制。
