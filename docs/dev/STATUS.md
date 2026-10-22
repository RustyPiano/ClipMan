# 项目状态（活文档）

> 记录当前状态、待办和已知问题，保持全文 ≤ 100 行。代码或发布状态变化时更新，过时条目直接删除。

**最后更新：2026-10-22**

## 当前状态

**v2.4.4 已于 2026-09-30 公开**，产品页已重新部署：QuickBar 多选与光标改为访达式（⇧ 点击选范围、方向键清空多选、删除后光标留在原位等），内容见 `release_notes_2.4.4.md`。v2.4.3 带来了列表重新设计和 macOS 来源应用图标。

文件摘要最多提供两个路径，允许重复路径正常显示，界面使用实际文件总数；设置保存拒绝非法值，存储目录读取失败可重新读取；纯文本写入共用，重复判断、异常包装、注释和文档已精简。QuickBar 忽略输入法确认按键，避免回车确认中文时触发粘贴。

## 最近验证

- 本机（2026-10-22，当前工作区）：Rust 107 项、前端 64 项、Chromium/WebKit 66 项、发布校验 6 项全部通过；lint、check、test:types、build、Clippy、rustfmt 和 Prettier 通过。性能基准按配置跳过，原生桌面交互及 Windows/Linux 运行未实测。
- main 的 CI（`7d6f386`）五个作业全部通过，包括 Windows 和 Linux 编译与测试。
- 本机 PATH 没有 `minisign`，运行发布校验测试和 `verify-release.py` 时用 `src-tauri/target/release-verification/tools/minisign`。
- 图片缩略图 `self-start`、类型图标和应用图标的实际观感需要用正式包在系统 WKWebView 中目测。
- Playwright 浏览器缓存保留在本机（用户决定，避免反复安装）。

## 待办（按优先级）

1. 用 v2.4.4 正式包回归：类型图标与应用图标显示、键盘/鼠标多选后回车合并粘贴、图片缩略图宽度、精简后的 capabilities、原生 `<dialog>`、globalPlain 剥离、文件写回失败报错、迁移期间继续采集、自定义目录不可用或剪贴板监听启动失败时报错退出。
2. 用正式包实机验证 Linux，回归 v2.4.3→v2.4.4 updater 路径，复测更新后辅助功能授权保留、文件 TCC 和多屏/Spaces。
3. QuickBar 交互待观察（2026-09-30 检查时发现，按实际使用情况再决定是否改）：
   - 有多选时 ⌘⌫ / ⌘P 只处理光标所在行，不处理勾选的行；访达会作用于全部选中项。批量删除没有撤销，要改需新增批量删除/置顶。
   - Esc 直接关闭并清空搜索词；可改成先清空搜索、再按一次关闭，与「先清空多选」一致，但关闭要多按一次。
   - 鼠标悬停会移动光标，⇧↑/↓ 选范围时鼠标划过别的行，下一次扩展会从鼠标所在行算起；悬停移动光标是为了让预览跟随鼠标。
4. Wave 4 候选（未排期）：Paste Stack 会话队列、Apple 公证。类型高亮、SQLCipher/同步继续 YAGNI。

## 已知问题 / 注意事项

- Linux 没有 X11（纯 Wayland、无 XWayland）时，剪贴板监听的 `run()` 返回错误，线程按 fast-fail 直接 panic，应用退出。
- Linux 的 arboard 不提供剪贴板序号，globalPlain 无法区分外部应用再次复制完全相同的文本及 HTML。
- Windows 剪贴板隐私标记检查仍手写 kernel32 FFI（`GlobalLock` 等），可改用 windows crate 的 `Win32_System_Memory`（`docs/archive/REVIEW-2026-07-07.md` #47）；由 CI `rust-windows` 保证能编译。
- 搜索最多展示 1000 条，已提示缩小范围；没有追加搜索分页。
- 应用排除仅 macOS 可用（支持 bundle ID / 兼容本地化名称）；Windows/Linux 已提示。
- 合并跳过图片，执行前显示跳过数量。
- 原图详情上限 16 MiB；文本/文件预览上限 1 MiB，均在 SQL 限制读取；复制/粘贴仍使用完整内容。
- 自定义目录里的数据库被 SQLite 判定损坏时，与默认目录一样隔离旧文件后新建空库。

## 背景资料指路

- 已完成的开发记录（v2.2 波次规格与验收、审核记录、QuickBar 改版记录）和 v2.0 重设计文档：`docs/archive/`，均带归档横幅，不作为当前指导。
- 发布流程：`.github/RELEASE_GUIDE.md`。
- 产品页（https://www.clipman.top）在 `RustyPiano/ClipMan-Page` 仓库，推送 main 后由 Vercel 部署。版本和下载链接在构建时读取最新发布，发版后需在 Vercel 重新部署（没有新提交时运行 `vercel redeploy <最新生产部署 URL> --target production`，项目名 `clip-man-page`）；QuickBar 界面变化后在该仓库运行 `npm run screenshots`，用本仓库前端和 WebKit 重新截图。
