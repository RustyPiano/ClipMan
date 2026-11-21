# 0016 AppKit 线程与启动顺序

- 状态：已采纳

## 决定

- 窗口显示、激活等 AppKit 操作都派发回主线程执行。前端 IPC 命令和 single-instance 回调都可能运行在 Tokio worker 上。
- single-instance 插件最先注册。再次启动应用（双击图标，或替换安装包之后）时，不产生和已有实例争抢全局快捷键、剪贴板监听的第二个实例，而是在已有实例里打开 QuickBar。
- 在 `setup()` 里设置 Accessory 激活策略（不显示 Dock 图标）。必须在 tao 创建 NSApplication 之后设置，更早设置会被重置为 Regular。

## 已知问题

采集线程直接读取前台应用并绘制应用图标，没有回到主线程，见 #3。
