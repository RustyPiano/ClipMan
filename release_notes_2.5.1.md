## ClipLeaf v2.5.1

### 更名为 ClipLeaf

- ClipMan 正式更名为 **ClipLeaf**，GitHub 仓库同步更名为 [RustyPiano/ClipLeaf](https://github.com/RustyPiano/ClipLeaf)，旧链接自动跳转。
- 历史记录、收藏、设置和 macOS 辅助功能授权全部保留，自动更新照常进行。
- 开机自启会自动迁移到新名称。Windows 和 Linux 更新时直接替换旧的 ClipMan 安装。macOS 通过自动更新升级时，“应用程序”中的文件名仍是 ClipMan.app，重新下载安装即为 ClipLeaf.app。

### 修复

- **macOS 全屏应用中无法唤出 QuickBar**：修复 v2.5.0 引入的回归。在其他应用的全屏窗口中按下全局快捷键时，QuickBar 现在会直接显示在当前全屏 Space 上。
- **搜索结果高亮被截断**：搜索命中位置靠后的长文本时，列表行会省略匹配词之前的大段内容，让匹配词和高亮出现在行首附近。

### Renamed to ClipLeaf

- ClipMan is now **ClipLeaf**, and the GitHub repository has moved to [RustyPiano/ClipLeaf](https://github.com/RustyPiano/ClipLeaf). Old links redirect automatically.
- History, favorites, settings, and the macOS Accessibility permission are all kept, and auto-update keeps working.
- Launch-at-login entries are migrated to the new name. On Windows and Linux the update replaces the old ClipMan installation. On macOS an auto-update keeps the file name ClipMan.app in Applications; a fresh download installs ClipLeaf.app.

### Fixes

- **QuickBar over fullscreen apps on macOS**: Fixed a v2.5.0 regression where the global shortcut did not show QuickBar while another app was in fullscreen. QuickBar now appears on the current fullscreen Space.
- **Search highlights cut off**: When a match sits deep inside long text, the list row now elides the preceding text so the highlighted match stays visible.
