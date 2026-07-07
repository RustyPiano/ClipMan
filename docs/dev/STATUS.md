# 项目状态（活文档）

> **这是 Agent 每个工作会话结束前必须更新的文件**（规则见 AGENTS.md「文档维护协议」）。
> 只记"当前是什么状态、接下来做什么"；做过的事的细节归档在 PLAN.md / release notes / git 历史，不要在这里堆积。
> 保持全文 ≤ 100 行；过时条目直接删除。

**最后更新：2026-07-07**

## 当前状态一句话

v2.2 全部改动（含双模型审核 49 项修复）**已提交到 `main`**（6 个提交）；版本已升到 **2.2.0**、release notes 已写；发布流程已全面自动化。下一步：（用户上机确认后）推 `v2.2.0` 标签触发构建 → 发布草稿。

## 工作区

- 干净（本次仅剩本文件的 STATUS 更新）。签名私钥目录 `ClipMan-signing/` 已加入 `.gitignore`，永不入库。

## 质量基线（改动必须保持全绿；本机无工具链，靠 CI）

```
cd src-tauri && cargo test               # 119 通过
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo fmt --check
bun run lint && bun run check            # 0 错误
bun test tests/                          # 47 通过
bun run build
```

## 待办（按优先级）

1. **切 v2.2.0 正式版**（自动化已就绪，任选其一）：
   - 本地：`git tag v2.2.0 && git push --follow-tags`（标签触发 `release.yml`：preflight 校验 → 4 平台构建 + 自签名 → **草稿** release）。
   - 或 Actions → **Prepare Release** 一键（需先配 `RELEASE_PAT` secret，否则标签不触发构建）。
   - 构建完成后：GitHub Releases 里编辑草稿 → **Publish**（更新器的 `latest.json` 只认已发布的非草稿 release）。
2. **上机复验**（推标签前）：v2.2 各特性（文件、富文本+⌥Enter、⌘Click 合并粘贴、托盘暂停、秘密跳过、忽略应用）与三处修复（文件粘贴、深色阴影、vacuum）。CI 已全绿但缺真机验证。
3. **发版可选增强**：给 ClipMan **完全磁盘访问**以获最佳文件粘贴体验；README 功能列表若有新特性再补（版本号/文件名已自动）。
4. Wave 4 候选（未排期）：Paste Stack 逐次粘贴、类型识别与语法高亮、SQLCipher 加密（同步前置）、搜索 1000 条截断提示、Apple 公证。

## 双模型代码审核（2026-07-07，已提交）

四路审毕约 55 条（规格：`docs/dev/REVIEW-2026-07-07.md`）：**49 条全部 fixed、0 跳过 0 失败**（含 anchor 竞态，测试先行，`anchor-race.test.ts`）。已随 v2.2 波次提交到 `main`。遗留决策：`group_name` 去留、短查询 4096 截断、#47 Windows FFI（现由 CI `rust-windows` 作业守护编译）。

## 已知问题 / 注意事项

- 搜索结果静默截断在 1000 条（storage.rs，无 UI 提示）
- `ignored_apps` 按应用本地化名称匹配（非 bundle id），跨语言环境有局限（有意为之的 v1 取舍）
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
