# 项目状态（活文档）

> 记录当前状态、待办和已知问题，保持全文 ≤ 100 行。代码或发布状态变化时更新，过时条目直接删除。

**最后更新：2026-11-27**

## 当前状态

- **未发布修复（已推送 main）**：修复 v2.5.2 审查发现的 10 个问题，本地质量基线全部通过，Windows/Linux 待 CI：
  - 数据库改名先改日志文件、最后改主库，中断后可继续；自定义数据路径先改写再移动目录；设置文件损坏或不可读时暂停采集（不再被 tauri-plugin-store 当作空设置）；开机自启先登记新项再删旧项；NSIS 旧版卸载失败时中止安装。
  - 隐私标记检查纳入剪贴板序号校验，Windows 无法检查标记时跳过采集；Linux 写剪贴板复用长期存在的实例。
  - 搜索切换分类会重新查询；链接分类改用后端按完整内容计算的 `isLink`；文件详情直接返回 `filePaths` 数组。
- **最新正式版：v2.5.2（已公开发布，完成更名）**：
  - `CI`、`Release`（含新增 `draft-release` 作业，四端构建/签名/`verify-release`）通过，`latest.json` 已切换为公开直链并复验；旧仓库地址的更新清单经跳转同样返回 2.5.2。
  - 变更：identifier 改为 `app.clipleaf.desktop`、可执行文件 `clipleaf`、数据库 `clipleaf.db`、新签名证书 `ClipLeaf Code Signing`（文件在被忽略的 `ClipLeaf-signing/`，需另行备份）；首次启动迁移旧数据目录、数据库、自定义路径、macOS 应用包名与开机自启（`docs/adr/0023-clipleaf-rename.md`）。macOS 用户需重新授权一次辅助功能。
  - v2.5.1：产品名与仓库改为 ClipLeaf，修复全屏 Space 唤出 QuickBar 与搜索高亮截断。
- **产品页**：本仓库 `website/`（Svelte + Vite），Vercel 项目 `clipleaf-page` 接入本仓库 `main`，域名 `clipleaf.app`（待在 Cloudflare 为 `@` 和 `www` 添加指向 Vercel 的 CNAME，关闭代理）；构建时读取最新 `latest.json` 决定下载版本，发版公开后需重新部署一次。旧产品页（`ClipMan-Page`、Vercel `clip-man-page`）已下线。
- **文档整理**：精简 `AGENTS.md`（去重目录表与 ADR 重复段落）、同步 ADR 0009/0012/0022 与 `.github/RELEASE_GUIDE.md`、清理 14 份已发布的历史 `release_notes_*.md`。

## 待办

1. 用 `v2.5.2` 正式包实机核对：macOS 从 ClipMan 2.5.x 自动更新后的数据迁移、`ClipLeaf.app` 改名、开机自启与辅助功能重新授权；Windows（MSI 与 NSIS 旧安装）和 Linux deb/rpm 的替换升级与自启迁移。
2. 查看上述修复在 CI 的 rust-windows / rust-linux 结果；实机核对 Windows 旧版卸载失败时安装中止、Linux X11（无剪贴板管理器）取用历史后粘贴。
3. 后续可在发布脚本或工作流中自动化草稿公开后的 `latest.json` 直链重写步骤，避免手工操作。

## 已知问题 / 注意事项

- 打开 QuickBar 会短暂激活 ClipLeaf，导致 Finder 文件重命名等失焦即退出的临时编辑框中断（详见 `docs/adr/0014-quickbar-macos-window.md`）。
- Linux 纯 Wayland（无 XWayland）下剪贴板监听无法启动；Linux 的 `globalPlain` 无法区分外部应用再次复制完全相同的文本及 HTML。
- 自定义数据目录启动时若数据库损坏会隔离备份为 `.corrupt-*` 并重建空库；若目录本身不可用则弹出致命错误并退出。
- 搜索最多展示 1000 条；应用排除目前仅支持 macOS；合并粘贴跳过图片；原图预览上限 16 MiB，文本/文件预览与在线编辑上限 1 MiB（复制/粘贴始终使用完整内容）。
