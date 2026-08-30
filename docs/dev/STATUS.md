# 项目状态（活文档）

> **这是 Agent 每个工作会话结束前必须更新的文件**（规则见 AGENTS.md「文档维护协议」）。
> 只记"当前是什么状态、接下来做什么"；做过的事的细节归档在 PLAN.md / release notes / git 历史，不要在这里堆积。
> 保持全文 ≤ 100 行；过时条目直接删除。

**最后更新：2026-08-30**

## 当前状态一句话

**v2.2.0 已发布；v2.2.1 发布候选已完成**：数据库恢复/迁移、前端错误状态/迁移交互、发布与跨平台 CI 已加固，正在等待 CI、标签构建与 Release 验收。

## 工作区

- v2.2.1 可靠性修复已提交，版本清单与发布说明已生成；用户报告 macOS / Windows 当前使用未发现问题。签名私钥目录 `ClipMan-signing/` 保持忽略。

## 质量基线（改动必须保持全绿；本机已有 cargo+bun，可本地跑，CI 复核）

```
cd src-tauri && cargo test               # 120 通过
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo fmt --check
bun run lint && bun run check            # 0 错误
bun run test:types
bun test tests/                          # 51 通过
bun run build
```

## 待办（按优先级）

1. 完成 v2.2.1 CI、标签构建与 Release 资产验收，然后用正式包验证 Linux 与 v2.1→v2.2/v2.2.1 updater 路径。
2. 做一次兼容依赖更新并复查 `bun audit`（当前 production audit 为 0，告警在开发/构建链）。
3. Wave 4 候选（未排期）：搜索 1000 条截断提示、Paste Stack 会话队列、Apple 公证。类型高亮、SQLCipher/同步继续 YAGNI。

## 代码审核记录

- **双模型审核（2026-07-07）**：约 55 条、49 fixed（规格 `docs/dev/REVIEW-2026-07-07.md`）。遗留决策：`group_name` 去留、短查询 4096 截断、#47 Windows FFI（CI `rust-windows` 守护）。
- **清晰度精简（2026-07-08）**：三路审查"能跑但不够清晰"，17 项确认全部落地（删 ~230 行：死 RAII 守卫、快捷键三标志状态机→直线流、搜索路径双保险、migration 重复列添加、测试驱动抽象等）。快捷键切换从 make-before-break 改为 break-before-make（毫秒级窗口，已接受）。审毕保留项（勿再"清理"）：`run_returned` 标志（护 250ms 竞态，有注释）、`StagedSqliteReplacement`（护目标目录已有库的数据丢失窗口）。
- **可靠性审核（2026-08-30）**：修复非损坏 DB 被误重置、quarantine 非原子、迁移覆盖目标库/清理误报成功、快捷键回滚、前端启动监听/错误状态/迁移交互，并加固发布标签输入与跨平台 CI；删除与 Bun 漂移的 npm 锁文件。

## 已知问题 / 注意事项

- 搜索结果静默截断在 1000 条（storage.rs，无 UI 提示）
- `ignored_apps` 仍按本地化名称匹配，且当前只有 macOS 能取得前台应用；Windows/Linux UI 尚未标注该限制
- 多选合并粘贴跳过图片项（v1 限制，有日志计数）
- 后端 `notify_copied` 仍有硬编码中文串（i18n 债务）
- 默认语言写死 zh-CN，不跟随系统

## 文档与自动化

- 发布自动化：`scripts/release.sh`（一键升级四清单+README）、`scripts/check-versions.sh`（一致性守护，CI+preflight 复用）、`prepare-release.yml`（Actions 一键，需 `RELEASE_PAT`）。指南见 `.github/RELEASE_GUIDE.md`。
- 文档体系与维护协议：见 `AGENTS.md`「Documentation map & maintenance protocol」（2026-07-07 建立）
- Hooks（`.claude/settings.json` + `.claude/hooks/`）：SessionStart 自动注入本文件；Stop 时若源码比本文件新则提醒更新（首次生效可能需要运行一次 `/hooks` 或重启会话）

## 背景资料指路

- 竞品分析与长期路线图：claude.ai artifact「ClipMan 盲点报告与路线图」（2026-07-07）
- v2.2 执行记录（波次、验收、偏差裁决）：`docs/dev/PLAN.md` + `SPEC-1..4`
- 6 月 v2.0 重设计的历史文档已归档：`docs/archive/`（带过期横幅，勿作当前指导）
