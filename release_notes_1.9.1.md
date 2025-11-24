# ClipMan v1.9.1 更新日志

> 本次更新优化了更新界面的用户体验，支持 Markdown 格式的 Release Notes 显示，并修复了版本号显示问题。

## ✨ 新功能

- **Markdown Release Notes**：更新日志现在支持完整的 Markdown 格式渲染。
    - 支持标题、列表、代码块、引用等富文本格式。
    - 自动适配浅色/深色主题。
    - 长链接自动换行，避免溢出。

## 🚀 改进与优化

### 用户体验 (UX)

- **版本号常驻显示**：当前版本号现在始终显示在"关于"页面，无需点击"检查更新"。
    - 使用 Tauri 的 `getVersion()` API 在页面加载时即获取版本号。
    - 提升了信息可达性和用户体验。

- **更新界面样式优化**：
    - 修复了深色模式下"安装更新"按钮样式问题。
    - 优化了 Markdown 内容的排版和间距。

## 🔧 内部 / 技术变更

- 新增 `marked@17.0.1` 依赖用于 Markdown 渲染。
- 创建了 `MarkdownContent.svelte` 组件，提供主题适配的样式系统。
- 重构了 `AboutSection.svelte`，分离版本号获取逻辑。

---

**完整更新记录**: https://github.com/RustyPiano/ClipMan/compare/v1.9.0...v1.9.1
