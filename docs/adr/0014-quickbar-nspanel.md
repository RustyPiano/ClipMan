# 0014 QuickBar 使用非激活 NSPanel

- 状态：已采纳

## 背景

QuickBar 打开时目标应用必须保持激活。否则访达会结束改名编辑，随后的 ⌘V 会在文件夹里生成 textClipping 文件。

## 决定

- NonactivatingPanel 样式只对 NSPanel 生效，而 tao 创建的是 NSWindow 子类。启动时用 `object_setClass` 把 QuickBar 换成 NSPanel 子类 `QuickBarNsPanel`，再设置 NonactivatingPanel 样式。
- 子类的 `canBecomeKeyWindow` 返回 true：无边框窗口默认不能成为 key window，而搜索框要接收键盘输入。
- 换类的安全前提：NSPanel 与 NSWindow 的实例大小相同（macOS 26 上都是 528 字节），新类不加实例变量。tao 的 TaoWindow 只多一个 focusable 变量，以及 canBecomeKey/MainWindow 和用于背景拖动的 sendEvent: 覆盖；QuickBar 不调用 `set_focusable`，也不拖动窗口。必须先换类再设置 NonactivatingPanel 样式，由 NSPanel 的 `setStyleMask:` 处理这个样式。
- 事后设置的样式不会更新 WindowServer 的激活标记，需要调用 AppKit 私有接口 `_setPreventsActivation:` 同步。
- 显示时调用 `orderFrontRegardless`，不调用 `window.set_focus()`：tao 会顺带调用 `activateIgnoringOtherApps`，让目标应用失去激活。
- 原生 QA（`scripts/native-qa`）断言打开和点击 QuickBar 后目标应用仍在前台，并覆盖访达改名时粘贴。QuickBar 不激活 ClipMan，QA 只能通过屏幕上的窗口判断它是否打开，并按高度排除菜单栏图标。
