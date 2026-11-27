# 项目状态（活文档）

> 记录当前状态、待办和已知问题，保持全文 ≤ 100 行。代码或发布状态变化时更新，过时条目直接删除。

**最后更新：2026-11-27**

## 当前状态

- **最新正式版 [v2.5.3](https://github.com/RustyPiano/ClipLeaf/releases/tag/v2.5.3)**：支持中断后继续数据库改名，修复自定义数据目录和开机自启迁移。设置损坏时暂停采集，Windows 无法检查隐私标记时跳过该次复制。搜索切换分类时重新查询，链接按完整内容识别，文件详情保留独立路径。
- [Release 工作流](https://github.com/RustyPiano/ClipLeaf/actions/runs/45970399049)通过三个系统的质量检查和四个平台构建。草稿的更新清单曾缺少 Windows 和 Linux，已按本版附件及签名补齐；公开清单含 11 个平台与安装格式条目，7 个更新包通过验签。
- 从 ClipMan 升级到 ClipLeaf 时，数据迁移和 macOS 辅助功能重新授权见 [ADR 0023](../adr/0023-clipleaf-rename.md)。
- **产品页**：[clipleaf.app](https://clipleaf.app) 使用本仓库 `website/`，Vercel 项目 `clipleaf-page` 跟随 `main` 部署。构建时读取最新公开更新清单生成下载链接。
- **文档整理**：精简 `AGENTS.md`（去重目录表与 ADR 重复段落）、同步 ADR 0009/0012/0022 与 `.github/RELEASE_GUIDE.md`、清理 14 份已发布的历史 `release_notes_*.md`。

## 待办

1. 用 `v2.5.3` 正式包实机核对：macOS 从 ClipMan 2.5.x 自动更新后的数据迁移、`ClipLeaf.app` 改名、开机自启与辅助功能重新授权；Windows（MSI 与 NSIS 旧安装）和 Linux deb/rpm 的替换升级与自启迁移。
2. 实机核对 Windows 旧版卸载失败时安装中止、Linux X11（无剪贴板管理器）取用历史后粘贴。
3. 后续可在发布脚本或工作流中自动化草稿公开后的 `latest.json` 直链重写步骤，避免手工操作。

## 已知问题 / 注意事项

- 打开 QuickBar 会短暂激活 ClipLeaf，导致 Finder 文件重命名等失焦即退出的临时编辑框中断（详见 `docs/adr/0014-quickbar-macos-window.md`）。
- Linux 纯 Wayland（无 XWayland）下剪贴板监听无法启动；Linux 的 `globalPlain` 无法区分外部应用再次复制完全相同的文本及 HTML。
- 自定义数据目录启动时若数据库损坏会隔离备份为 `.corrupt-*` 并重建空库；若目录本身不可用则弹出致命错误并退出。
- 搜索最多展示 1000 条；应用排除目前仅支持 macOS；合并粘贴跳过图片；原图预览上限 16 MiB，文本/文件预览与在线编辑上限 1 MiB（复制/粘贴始终使用完整内容）。
