# 0015 QuickBar 窗口尺寸、位置与阴影

- 状态：已采纳

## 决定

- 尺寸和位置按光标所在的显示器计算，取不到时依次退到窗口当前所在的显示器和主显示器。`current_monitor()` 只反映窗口上次所在的位置，多屏时会把 QuickBar 放到错误的屏幕。尺寸和居中都按目标显示器的缩放比例计算，混合 DPI 时位置也正确。
- 阴影只用 macOS 原生窗口阴影。窗口透明时，WindowServer 按渲染出的 alpha 形状绘制阴影，所以面板必须完全不透明，只画边框，不加 CSS 阴影和 backdrop-blur。面板周围的半透明像素会成为窗口形状的一部分，阴影会变成贴着圆角的灰色光晕。只有 QuickBar 窗口的 html 和 body 背景透明，设置窗口保持不透明。
- 显示窗口和改变尺寸后重新计算阴影：在另一块显示器上改变尺寸后，alpha 形状可能还是旧的。
- Windows 保持 `shadow: false`：DWM 阴影沿矩形边框绘制，不跟随面板形状。
- Windows 上 Tauri 使用的 `windows` crate 比本 crate 新（0.61 与 0.58），`webview_hwnd` 通过裸句柄转换 HWND。
