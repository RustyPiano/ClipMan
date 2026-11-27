# 项目状态（活文档）

> 记录当前状态、待办和已知问题，保持全文 ≤ 100 行。代码或发布状态变化时更新，过时条目直接删除。

**最后更新：2026-11-27**

## 当前状态

- **最新正式版 [v2.6.0](https://github.com/RustyPiano/ClipLeaf/releases/tag/v2.6.0)**：首次启动欢迎页与空列表引导；后台每日检查更新，托盘和 QuickBar 提示，安装显示进度、装完由用户重启；设置即时保存，托盘菜单并入“常规”，“存储”页显示用量、导出/导入收藏、删除旧备份；日志写入系统日志目录，“关于”页可复制诊断信息；`secure_delete`、FTS5 secure-delete 与清空后截断 WAL；Linux X11 识别密码管理器标记并用 XFixes 计数剪贴板序号；密钥规则 8 → 14 条；macOS/X11 剪贴板轮询 500 → 100 ms。设计见 ADR 0024。
- [Release 工作流](https://github.com/RustyPiano/ClipLeaf/actions/runs/46004639716)通过三个系统的质量检查和四个平台构建；公开的更新清单含 11 个平台与安装格式条目，已改为公开直链，7 个更新包通过验签；产品页已重新部署到 2.6.0。
- 从 ClipMan 升级到 ClipLeaf 时，数据迁移和 macOS 辅助功能重新授权见 [ADR 0023](../adr/0023-clipleaf-rename.md)。
- **产品页**：[clipleaf.app](https://clipleaf.app) 使用本仓库 `website/`，Vercel 项目 `clipleaf-page` 跟随 `main` 部署。构建时读取最新公开更新清单生成下载链接。

## 待办

1. 用 `v2.6.0` 正式包实机核对：全新安装时欢迎页出现并在授权辅助功能后自动回到欢迎页（`show_onboarding_when_trusted`）；后台更新提示、下载进度和“立即重启”要等下一个版本发布后从 2.6.0 升级验证；Linux X11 密码标记和 XFixes 剪贴板序号只在容器里验证过，需在真实 KDE + KeePassXC 上确认。
2. Windows 代码签名：申请 SignPath Foundation 开源项目免费证书，通过后在 `release.yml` 接入签名步骤。
3. 用 `v2.5.3` 正式包实机核对：macOS 从 ClipMan 2.5.x 自动更新后的数据迁移、`ClipLeaf.app` 改名、开机自启与辅助功能重新授权；Windows（MSI 与 NSIS 旧安装）和 Linux deb/rpm 的替换升级与自启迁移。
4. 实机核对 Windows 旧版卸载失败时安装中止、Linux X11（无剪贴板管理器）取用历史后粘贴。
5. 后续可在发布脚本或工作流中自动化草稿公开后的 `latest.json` 直链重写步骤，避免手工操作。

## 已知问题 / 注意事项

- 打开 QuickBar 会短暂激活 ClipLeaf，导致 Finder 文件重命名等失焦即退出的临时编辑框中断（详见 `docs/adr/0014-quickbar-macos-window.md`）。
- Linux 纯 Wayland（无 XWayland）下剪贴板监听无法启动。
- `tauri::Builder` 创建之前的日志（ClipMan 身份迁移的信息日志）不写入日志文件；迁移失败的错误在 `setup()` 里记录，会写入。
- 自定义数据目录启动时若数据库损坏会隔离备份为 `.corrupt-*` 并重建空库；若目录本身不可用则弹出致命错误并退出。
- 搜索最多展示 1000 条；应用排除目前仅支持 macOS；合并粘贴跳过图片；原图预览上限 16 MiB，文本/文件预览与在线编辑上限 1 MiB（复制/粘贴始终使用完整内容）。
