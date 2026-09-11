# 项目状态（活文档）

> **这是 Agent 每个工作会话结束前必须更新的文件**（规则见 AGENTS.md「文档维护协议」）。
> 只记"当前是什么状态、接下来做什么"；做过的事的细节归档在 PLAN.md / release notes / git 历史，不要在这里堆积。
> 保持全文 ≤ 100 行；过时条目直接删除。

**最后更新：2026-09-11**

## 当前状态一句话

**v2.3.0 已发布；工作区有未提交改动**：QuickBar 行信息密度回归 v2.2 风格（方案 B）+ 开发产物瘦身，全部门禁绿。

## 工作区

- QuickBar 方案 B（未提交）：行高 4rem→5.5rem（`ROW_HEIGHT_REM` 常量）三段式行——标签行 + 内容预览（文本两行 / 文件两个路径+目录 / 图片行内缩略图，有标签时压缩为一行）+ 元数据行；恢复选中高亮条、多选徽章、复制 ✓ 行内反馈；行操作按钮在行按钮之外（嵌套按钮会把操作点击冒泡成粘贴，见 AGENTS.md gotcha），覆盖元数据行右侧预留区（`pr-[8.75rem]`），文字永不被盖。虚拟化/ID 选择/请求合并保留。
- 三路 subagent 并行审核后修复（2026-09-11）：①操作按钮改内联 24px 尺寸（Button 的 `h-9` 会盖过 class 里的 `h-6`，36px 按钮会超出预留区盖字）；②`fileBasename/fileDirname/looksLikeDirectory` 支持 `\` 分隔符并抽到 `clip-items.ts`（原实现丢了 Windows 路径拆分）；③action 列表改稳定 `id` 键（Pin/Unpin 文案翻转不再销毁聚焦按钮）；另修 pointer-events、+N 计数、kbd 空格、`File as FileIcon`、注释统一英文。e2e 新增：+N 徽章/Windows 路径/标签行/元数据不遮挡/复制 ✓ 断言，40 项全绿。
- 磁盘瘦身（未提交）：`[profile.dev.package."*"] debug = false` 后全量重编译，`src-tauri/target` 12 GB→1.9 GB（release 产物一并清除）；Playwright 浏览器缓存**保留**（用户决定不再反复装卸，避免重复写盘）。
- v2.3.0 已公开发布（17 个附件，Windows / Linux / Intel Mac / Apple Silicon），GitHub Release 与 updater 最新入口均指向 2.3.0；签名私钥目录 `ClipMan-signing/` 保持忽略。

## 质量基线（改动必须保持全绿；本机已有 cargo+bun，可本地跑，CI 复核）

```
cd src-tauri && cargo test               # 132 通过，1 个手动性能基准 ignored
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo fmt --check
bun run lint && bun run check            # 0 错误
bun run test:types
bun test tests/                          # 56 通过
bun run build
bun run test:ui                          # Chromium + WebKit，36 项
```

## 待办（按优先级）

1. 用 v2.3.0 正式包实机验证 Linux，并回归 v2.2.1→v2.3.0 updater 路径。
2. 发布前用正式签名包复测更新后辅助功能授权保留、文件 TCC 和多屏/Spaces。
3. Wave 4 候选（未排期）：Paste Stack 会话队列、Apple 公证。类型高亮、SQLCipher/同步继续 YAGNI。

## 代码审核记录

- **全仓审查与修复（2026-09-07）**：B1–B9、O1、T1–T4、K1、较小交互问题与精简项全部落实；依赖全量审计为 0。见 `AUDIT-2026-09-07.md`。

- **双模型审核（2026-07-07）**：约 55 条、49 fixed（规格 `docs/dev/REVIEW-2026-07-07.md`）。遗留决策：`group_name` 去留、#47 Windows FFI（CI `rust-windows` 守护）。
- **清晰度精简（2026-07-08）**：三路审查"能跑但不够清晰"，17 项确认全部落地（删 ~230 行：死 RAII 守卫、快捷键三标志状态机→直线流、搜索路径双保险、migration 重复列添加、测试驱动抽象等）。快捷键切换从 make-before-break 改为 break-before-make（毫秒级窗口，已接受）。审毕保留项（勿再"清理"）：`run_returned` 标志（护 250ms 竞态，有注释）、`StagedSqliteReplacement`（护目标目录已有库的数据丢失窗口）。
- **可靠性审核（2026-08-30）**：修复非损坏 DB 被误重置、quarantine 非原子、迁移覆盖目标库/清理误报成功、快捷键回滚、前端启动监听/错误状态/迁移交互，并加固发布标签输入与跨平台 CI；删除与 Bun 漂移的 npm 锁文件。

## 已知问题 / 注意事项

- 搜索最多展示 1000 条，已提示缩小范围；没有追加搜索分页。
- 应用排除仅 macOS 可用（支持 bundle ID / 兼容本地化名称）；Windows/Linux 已提示。
- 合并仍跳过图片，执行前显示跳过数量。
- 原图详情上限 16 MiB；文本/文件预览上限 1 MiB，均在 SQL 限制读取；复制/粘贴仍使用完整内容。
- macOS 27 单屏 release QA 包实测通过：启动/单实例/快捷键/中文采集/Esc/无权限降级、授权后 8 轮真实粘贴（含搜索立即回车）、仅复制、大文本预览、图片恢复、Finder 文件粘贴；后续发现并修复同秒排序，最终脚本 8 轮取用/小图复用/托盘故障保留均通过。已恢复剪贴板并退出 QA；正式签名更新、多屏/Spaces、其他系统及受保护目录 TCC 未测。
- 本轮重建的最终 QA 包通过完整原生回归：8 轮真实自动粘贴（4 轮含干扰项后搜索立即回车）、小图 PNG 复用、托盘读失败保留、剪贴板恢复与 QA 退出；证据 `/var/folders/tv/6cy46rxx6tq18h17n9wh7qhh0000gn/T/clipman-native-qa-gg_n9s_p/`。首次授权后复跑曾丢失 1 次合成全局热键事件，未改代码或放宽断言，原样重跑连续 8 轮通过；日常 ClipMan 已重新启动。

## 文档与自动化

- 发布自动化：`scripts/release.sh`（一键升级四清单+README）、`scripts/check-versions.sh`（一致性守护，CI+preflight 复用）、`prepare-release.yml`（Actions 一键，需 `RELEASE_PAT`）。指南见 `.github/RELEASE_GUIDE.md`。
- 文档体系与维护协议：见 `AGENTS.md`「Documentation map & maintenance protocol」（2026-07-07 建立）
- Hooks（`.claude/settings.json` + `.claude/hooks/`）：SessionStart 自动注入本文件；Stop 时若源码比本文件新则提醒更新（首次生效可能需要运行一次 `/hooks` 或重启会话）

## 背景资料指路

- 竞品分析与长期路线图：claude.ai artifact「ClipMan 盲点报告与路线图」（2026-07-07）
- v2.2 执行记录（波次、验收、偏差裁决）：`docs/dev/PLAN.md` + `SPEC-1..4`
- 6 月 v2.0 重设计的历史文档已归档：`docs/archive/`（带过期横幅，勿作当前指导）
