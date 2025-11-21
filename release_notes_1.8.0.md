# ClipMan v1.8.0 更新日志
> 全新的设置体验、智能快捷键录入和完全掌控数据存储位置

## ✨ 新功能

### 📂 自定义数据存储位置
- **完全掌控您的数据**：新增数据迁移功能，让您自由选择 ClipMan 数据的存储位置
    - 支持将数据库、加密密钥等所有数据文件迁移到任意位置
    - 提供"保留原文件"或"删除原文件"两种迁移模式
    - 完整的迁移验证机制：文件大小校验、数据完整性检查
    - 自动重启剪贴板监控，确保迁移后功能正常
    - 贴心的错误处理：权限检查、路径验证、同路径检测
- **在 Finder 中打开**：一键在文件管理器中打开当前数据目录，方便查看和管理

### 🎹 智能快捷键录入系统
- **键盘录入模式**：告别手动输入，直接按下键盘组合即可设置快捷键
    - 点击"录入"按钮激活，实时显示按下的修饰键（如 ⌘ + ⇧ + ?）
    - 录入期间自动暂停全局快捷键，避免误触发应用窗口
    - 智能冲突检测：尝试设置当前快捷键时给出友好提示
    - ESC 随时取消录入，2秒后自动关闭警告提示
- **美观的按键显示**：根据操作系统显示对应的符号
    - macOS: ⌘ (Command)、⇧ (Shift)、⌥ (Option)、⌃ (Control)
    - Windows/Linux: Ctrl、Shift、Alt
    - 每个按键采用渐变背景、边框阴影的精美键盘样式
- **快速预设**：提供常用快捷键按钮（Cmd+Shift+V、Cmd+Shift+C、Alt+V），一键切换
- **高级选项**：折叠式手动输入区域，适合高级用户精确配置

### 🧹 清空所有历史
- 在"剪贴板"设置中新增"清空所有历史"按钮
- 可删除所有剪贴板记录（包括置顶项）
- 带有二次确认对话框，防止误操作

### 🚀 开机自启动
- 新增"开机自启动"设置选项
- 支持系统启动时自动运行 ClipMan（静默启动，最小化到托盘）
- 使用系统原生 API 确保可靠性

## � 改进与优化

### 用户体验 (UX)
- **全新的模块化设置界面**：将庞大的设置页面拆分为清晰的分类
    - 侧边栏导航：常规、剪贴板、托盘菜单、数据存储、关于
    - 每个设置组都是独立的 Svelte 组件，代码更清晰、可维护性更高
    - 平滑的动画过渡，提升视觉体验
- **深色模式优化**：修复切换开关在深色模式下的显示问题
    - 使用 CSS 自定义属性 `var(--muted)` 替代 Tailwind 类
    - 确保所有设置项在深色模式下清晰可见
- **更好的视觉反馈**：
    - 录入模式下背景高亮和脉冲动画
    - 彩色警告提示（橙色）
    - 加载状态显示

### 性能提升
- **后端命令优化**：移除3个冗余命令，精简 API
    - 删除 `get_pinned_clips`（功能重复，前端已实现筛选）
    - 删除 `get_current_data_path`（前端可自行推导）
    - 删除 `choose_data_folder`（已被更好的对话框方案替代）
- **代码模块化**：设置页面拆分为 6 个组件，代码量减少 40%+
    - `GeneralSettings.svelte` - 常规设置（148行）
    - `ClipboardSettings.svelte` - 剪贴板设置（147行）
    - `TraySettings.svelte` - 托盘菜单设置（94行）
    - `StorageSettings.svelte` - 数据存储设置（95行）
    - `AboutSection.svelte` - 关于和更新（149行）
    - `Sidebar.svelte` - 导航侧边栏（40行）

## 🐛 缺陷修复

- 修复 `update_settings` 命令参数名不匹配问题（`newSettings` → `settings`）
- 修复数据迁移命令名称错误（`migrate_data` → `migrate_data_location`）
- 修复深色模式下切换开关背景色全白的显示问题
- 修复自启动 API 调用错误（`autostart()` → `autolaunch()`）
- 修复迁移测试用例的目录创建问题
- 修复启动时未正确使用自定义数据路径的关键问题
- 修复无障碍性问题：为数据路径输入框关联标签
- 修复迁移后剪贴板监控的重启逻辑

## 🔧 内部 / 技术变更

- **新增模块**：`src-tauri/src/migration.rs`（119行）
    - 完整的数据迁移逻辑
    - 文件完整性验证
    - 权限检查和错误处理
    - 单元测试覆盖
- **新增后端命令**：
    - `disable_global_shortcut` - 临时禁用全局快捷键
    - `enable_global_shortcut` - 重新启用全局快捷键
    - `clear_all_history` - 清空所有剪贴板历史
    - `open_folder` - 在系统文件管理器中打开文件夹
    - `migrate_data_location` - 迁移数据存储位置
- **Settings 结构扩展**：
    - 新增 `custom_data_path: Option<String>` 字段
    - 新增 `enable_autostart: bool` 字段
- **依赖更新**：
    - 添加 `tauri-plugin-autostart = "2.0.1"` 用于开机自启动
    - Cargo.lock 更新 121 处依赖版本
- **前后端 API 完全对齐**：所有 `invoke` 调用都已验证与后端命令匹配

## � 代码统计

| 指标 | v1.7.0 | v1.8.0 | 变化 |
|------|--------|--------|------|
| 前端组件数 | 1个大文件 | 6个模块化组件 | +500% 可维护性 |
| settings/+page.svelte | ~780行 | ~350行 | -55% 代码量 |
| 后端命令数 | 15个 | 16个 | 移除3个冗余，新增4个实用 |
| 新增功能模块 | - | migration.rs (119行) | 数据迁移支持 |

---

**完整更新记录**: https://github.com/yourusername/clipman/compare/v1.7.0...v1.8.0

## 🎁 升级提示

- ✅ 与 v1.7.0 完全兼容，所有用户数据和设置自动保留
- 🔒 首次启动时会使用您现有的数据路径
- 📂 可随时通过"数据存储"设置迁移到新位置
- 🚀 建议启用"开机自启动"以获得最佳体验

## 📥 下载地址

- **macOS (Apple Silicon)**: ClipMan_1.8.0_aarch64.dmg
- **macOS (Intel)**: ClipMan_1.8.0_x64.dmg
- **Windows**: ClipMan_1.8.0_x64_en-US.msi
- **Linux**: ClipMan_1.8.0_amd64.AppImage

[GitHub Releases](https://github.com/Kiaana/ClipMan/releases/tag/v1.8.0)

## 💬 反馈渠道

发现问题或有新想法？欢迎通过以下方式联系我们：
- **GitHub Issues**: https://github.com/yourusername/clipman/issues
- **讨论区**: https://github.com/yourusername/clipman/discussions

---

感谢使用 ClipMan! 🎉
