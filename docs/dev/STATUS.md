# 项目状态（活文档）

> **这是 Agent 每个工作会话结束前必须更新的文件**（规则见 AGENTS.md「文档维护协议」）。
> 只记"当前是什么状态、接下来做什么"；做过的事的细节归档在 docs/archive / release notes / git 历史，不要在这里堆积。
> 保持全文 ≤ 100 行；过时条目直接删除。

**最后更新：2026-09-30**

## 当前状态一句话

**v2.4.2 草稿已生成，等用户确认后 Publish**：标签在 `bc5550c` 上重打后 Release 全部通过（含四平台打包和 `verify-release`，17 个附件）（修复 QuickBar 多选：点击勾选后回车反复勾选、新增 ⇧↑/↓ 键盘多选）。原 v2.4.2 标签提交里的 e2e 测试 `closing the session cancels Enter…` 反复失败：`fill()` 与 Enter 分开发送时，慢机器上 30ms 防抖加 90ms 模拟搜索会先完成，测试走不到取消路径；已改为输入、Enter、Escape 同一任务派发并断言搜索仍在进行。上一公开版本 v2.4.1（2026-09-30）。

## main 上未发布的改动（v2.4.2 之后，本地提交，尚未推送）

- QuickBar 行重新设计（结构说明见 AGENTS.md「QuickBar rows」）：行高 4.5rem，40px 内容区加 24px 元信息行；左侧细线类型图标按内容种类区分并兼任圆形多选框；操作按钮只在悬停或焦点时出现；正文比例字体，代码类文本等宽；时间显示为今天的时刻、昨天、日期；预览头部加字符数或文件数；搜索框与面板切换同高。
- 来源应用图标：`app_icons` 表、`get_app_icon` 命令、`app-icon-saved` 事件，macOS 采集时渲染。旧记录要等该应用在新版本里再复制一次才有图标。
- 删除 STATUS.md 的注入和检查 hook。

## 最近验证

- 本机：cargo fmt / clippy `-D warnings` / test（105 项，含 macOS 真实图标渲染）；lint、check、test:types、`bun test tests/`、build、prettier、`bun run test:ui`（Chromium + WebKit 52 项）。
- 新的 Rust 代码在 Windows、Linux 上能否编译只能由 CI 确认：本机交叉编译缺少 `ring` 需要的 C 头文件。
- 图片缩略图 `self-start`、类型图标和应用图标的实际观感需要用正式包在系统 WKWebView 中目测。
- Playwright 浏览器缓存保留在本机（用户决定，避免反复安装）。

## 待办（按优先级）

1. 推送 main 让 CI 验证 Windows/Linux 编译；v2.4.2 草稿经用户确认后 Publish；QuickBar 重新设计随下一个版本发布，发布前把类型图标和应用图标写进 README 功能列表；发布后在 Vercel 重新部署产品页，并在 `ClipMan-Page` 运行 `npm run screenshots` 更新截图。
2. 用 v2.4.2 正式包回归：键盘/鼠标多选后回车合并粘贴、图片缩略图宽度、精简后的 capabilities、原生 `<dialog>`、globalPlain 剥离、文件写回失败报错、迁移期间继续采集、自定义目录不可用或剪贴板监听启动失败时报错退出。
3. 用正式包实机验证 Linux，回归 v2.3.0→v2.4.2 updater 路径，复测更新后辅助功能授权保留、文件 TCC 和多屏/Spaces。
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

- 竞品分析与长期路线图：claude.ai artifact「ClipMan 盲点报告与路线图」（2026-07-07）。
- 已完成的开发记录（v2.2 波次规格与验收、审核记录、QuickBar 改版记录）和 v2.0 重设计文档：`docs/archive/`，均带归档横幅，不作为当前指导。
- 发布流程：`.github/RELEASE_GUIDE.md`。
- 产品页（https://www.clipman.top）在 `RustyPiano/ClipMan-Page` 仓库，推送 main 后由 Vercel 部署。版本和下载链接在构建时读取最新发布，发版后需在 Vercel 重新部署；QuickBar 界面变化后在该仓库运行 `npm run screenshots`，用本仓库前端和 WebKit 重新截图。
