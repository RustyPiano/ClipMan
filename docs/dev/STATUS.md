# 项目状态（活文档）

> 记录当前状态、待办和已知问题，保持全文 ≤ 100 行。代码或发布状态变化时更新，过时条目直接删除。

**最后更新：2026-11-27**

## 当前状态

- **最新正式版：v2.5.1（已公开发布，更名 ClipLeaf）**：
  - `CI`、`Release`（四端构建/签名/`verify-release`）通过，`latest.json` 已切换为公开直链并复验；旧仓库地址的更新清单经 GitHub 跳转同样返回 2.5.1。发布时两个打包作业各建了一个草稿，已手工合并后复验，并在 `release.yml` 新增 `draft-release` 作业防止再次发生（下次发版首次验证）。
  - 变更：产品名、界面文字、安装包和仓库（`RustyPiano/ClipLeaf`）改用 ClipLeaf，identifier、数据文件、签名证书和可执行文件名保持不变（`docs/adr/0023-clipleaf-rename.md`）；同时修复 macOS 全屏 Space 上无法唤出 QuickBar 的回归，以及搜索高亮被行尾截断。
- **产品页**：本仓库 `website/`（Svelte + Vite），Vercel 项目 `clipleaf-page` 接入本仓库 `main`，域名 `clipleaf.app`；构建时读取最新 `latest.json` 决定下载版本，发版公开后需重新部署一次。旧产品页（`ClipMan-Page`、Vercel `clip-man-page`）已下线。
- **文档整理**：精简 `AGENTS.md`（去重目录表与 ADR 重复段落）、同步 ADR 0009/0012/0022 与 `.github/RELEASE_GUIDE.md`、清理 14 份已发布的历史 `release_notes_*.md`。

## 待办

1. 用 `v2.5.1` 正式包在 macOS 实机核对中文输入法候选窗、全屏 Space 唤出、自动更新安装及复制/粘贴焦点恢复；在 Windows（MSI 与 NSIS 旧安装）和 Linux deb 上核对从 ClipMan 升级的替换与开机自启迁移。
2. 后续可在发布脚本或工作流中自动化草稿公开后的 `latest.json` 直链重写步骤，避免手工操作。

## 已知问题 / 注意事项

- 打开 QuickBar 会短暂激活 ClipLeaf，导致 Finder 文件重命名等失焦即退出的临时编辑框中断（详见 `docs/adr/0014-quickbar-macos-window.md`）。
- Linux 纯 Wayland（无 XWayland）下剪贴板监听无法启动；Linux 的 `globalPlain` 无法区分外部应用再次复制完全相同的文本及 HTML。
- 自定义数据目录启动时若数据库损坏会隔离备份为 `.corrupt-*` 并重建空库；若目录本身不可用则弹出致命错误并退出。
- 搜索最多展示 1000 条；应用排除目前仅支持 macOS；合并粘贴跳过图片；原图预览上限 16 MiB，文本/文件预览与在线编辑上限 1 MiB（复制/粘贴始终使用完整内容）。
