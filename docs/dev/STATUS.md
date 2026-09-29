# 项目状态（活文档）

> **这是 Agent 每个工作会话结束前必须更新的文件**（规则见 AGENTS.md「文档维护协议」）。
> 只记"当前是什么状态、接下来做什么"；做过的事的细节归档在 docs/archive / release notes / git 历史，不要在这里堆积。
> 保持全文 ≤ 100 行；过时条目直接删除。

**最后更新：2026-09-30**

## 当前状态一句话

**v2.4.1 草稿已生成，等待 Publish**：新发布流程第一次实际运行，preflight、quality、四平台打包和 `verify-release` 全部通过，草稿有 17 个附件。内容见 `release_notes_2.4.1.md`（三种粘贴格式、5.5rem 行、去除 fallback 的清理、发布流程加固、WebKit 行显示修复）。v2.4.0 未发布（标签已删除）：标签提交里的 e2e 测试 `closing the session cancels Enter…` 在 CI 的 WebKit 上两次因时序失败（Escape 晚于 90ms 模拟搜索），已在 `a1d06b5` 改为同一任务内派发按键。

## 最近验证

- 发版前本机：lint、check、test:types、`bun test tests/`（59 项）、build、prettier、`bun run test:ui`（Chromium + WebKit 48 项）。Rust 自 `cfd1234` 起只改了版本号，由 CI 覆盖。
- WebKit 行修复里的图片 `self-start` 在 Playwright WebKit 下本来就不复现，只在系统 WKWebView 中出现，需要用正式包目测。
- Playwright 浏览器缓存保留在本机（用户决定，避免反复安装）。

## 待办（按优先级）

1. 在 GitHub 上 Publish v2.4.1 草稿；在 Vercel 重新部署产品页，并在 `ClipMan-Page` 运行 `npm run screenshots` 更新截图。
2. 用 v2.4.1 正式包回归：图片缩略图宽度、精简后的 capabilities、原生 `<dialog>`、globalPlain 剥离、文件写回失败报错、迁移期间继续采集、自定义目录不可用或剪贴板监听启动失败时报错退出。
3. 用正式包实机验证 Linux，回归 v2.3.0→v2.4.1 updater 路径，复测更新后辅助功能授权保留、文件 TCC 和多屏/Spaces。
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
