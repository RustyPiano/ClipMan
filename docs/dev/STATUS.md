# 项目状态（活文档）

> **这是 Agent 每个工作会话结束前必须更新的文件**（规则见 AGENTS.md「文档维护协议」）。
> 只记"当前是什么状态、接下来做什么"；做过的事的细节归档在 docs/archive / release notes / git 历史，不要在这里堆积。
> 保持全文 ≤ 100 行；过时条目直接删除。

**最后更新：2026-09-29**

## 当前状态一句话

**最新发布版本为 v2.3.0**；main 上有两个未推送的提交，包含发布流程加固和过度设计与防御性代码清理，2026-09-29 本机全部质量关卡通过，远端 CI 和新发布流程尚未运行。

## 最近提交（未推送）

- 发布流程：版本预检查、共用 CI、`verify-release.py` 下载附件并用 minisign 校验更新签名和历史公钥；无用代码、`serde_bytes` 依赖和 47 个生成图标已删除。
- 2026-09-29 清理（按全仓审查逐项修改）：
  - 删除：剪贴板监听的停止/重启和轮询 fallback、迁移目标的替换逻辑、从未发布过的旧设置键、`group_name`、⌥回车后的延迟富文本清理、文件写回失败时改写路径文本、自定义目录不可用时退回默认目录、`safe_lock`、Markdown 渲染（更新说明改为原文显示，移除 marked）、`hasTauriRuntime` 及同类判断、手写焦点陷阱（改用原生 `<dialog>`）。
  - IPC：`paste_clip` 的 plain 必填；`paste_clips` 不再传 separator；`check_clipboard_permission` 失败时抛出错误；列表项不再有 `groupName`。
  - 数据库每次打开时把换行格式的文件记录转为 JSON；界面语言以后端 `settings.locale` 为准。
  - 剪贴板监听启动失败时与数据存储初始化失败一样，弹出错误对话框后退出；读取版本号、打开辅助功能设置失败时以提示显示错误；`vite.config.js` 改读 Tauri v2 的 `TAURI_ENV_DEBUG`，debug 构建不再压缩并生成 sourcemap。
  - capabilities 只保留 `core:default`、`core:window:allow-hide`、`dialog:default`；CI 和发布工作流去掉 `dtolnay/rust-toolchain`，由 rustup 按 `rust-toolchain.toml` 安装 1.96.0。
  - 8 份已完成的开发文档移入 `docs/archive/`；README、发布指南、AGENTS.md 同步修改。
- 本机验证：cargo fmt / clippy `-D warnings` / test（103 项，1 个手动基准忽略）/ build；lint、check、test:types、`bun test tests/`（59 项）、build、prettier；`bun run test:ui`（Chromium + WebKit 48 项）；发布校验测试 6 项。命令见 AGENTS.md「Testing & quality gates」。
- Playwright 浏览器缓存保留在本机（用户决定，避免反复安装）。

## 待办（按优先级）

1. 推送后确认 CI：去掉 dtolnay 后 1.96.0 能自动安装，`rust-windows`、`rust-linux` 能编译本机未编译的 Windows/Linux 分支。
2. 在真实应用里回归本轮行为变化：精简后的 capabilities、原生 `<dialog>`、globalPlain 剥离、文件写回失败报错、迁移期间继续采集、自定义目录不可用或剪贴板监听启动失败时报错退出。
3. 用 v2.3.0 正式包实机验证 Linux，并回归 v2.2.1→v2.3.0 updater 路径。
4. 发布前用正式签名包复测更新后辅助功能授权保留、文件 TCC 和多屏/Spaces。
5. Wave 4 候选（未排期）：Paste Stack 会话队列、Apple 公证。类型高亮、SQLCipher/同步继续 YAGNI。

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
