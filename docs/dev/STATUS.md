# 项目状态（活文档）

> **这是 Agent 每个工作会话结束前必须更新的文件**（规则见 AGENTS.md「文档维护协议」）。
> 只记"当前是什么状态、接下来做什么"；做过的事的细节归档在 PLAN.md / release notes / git 历史，不要在这里堆积。
> 保持全文 ≤ 100 行；过时条目直接删除。

**最后更新：2026-07-07**

## 当前状态一句话

v2.1.0 已发布；v2.2 的三波多 Agent 开发（14 项任务）已全部完成并通过终验，**改动尚未提交**，待用户手动上机验证。

## 工作区（未提交改动）

- v2.2 全部改动：采集管线单一代表格式、`ContentType::Files`、`html` 富文本列、键集分页、`secrets.rs` 秘密检测、应用忽略 + 暂停采集、多选合并粘贴（`paste_clips`）、启动降级恢复、CI（`.github/workflows/ci.yml`）、auto_vacuum、大小上限、深色模式阴影修复（原生 NSWindow 阴影）
- 更早的 WIP（同样未提交）：`source_app` 来源应用追踪、`now.svelte.ts` 共享时钟
- 规模：约 24 文件 +3031/-261；执行记录与验收结论见 `docs/dev/PLAN.md`

## 质量基线（改动必须保持全绿）

```
cd src-tauri && cargo test               # 117 通过
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo fmt --check
bun run lint && bun run check            # 0 错误
bun test tests/                          # 42 通过
bun run build
```

## 待办（按优先级）

1. **用户重新验证三个修复**（2026-07-07 晚，用户首轮实测反馈后修复）：
   - **文件粘贴：最终根因已实锤并修复（macOS 26 pboard 静默丢弃无权限文件 URL）**。协调者在 LaunchServices 上下文全自动复现：桌面文件 → `writeObjects` 返回 true 但 `items=0` 被静默丢弃；`/Users/Shared` 文件 → 完整链路粘贴成功落地。终端启动能用是因为继承了终端的磁盘访问权。修复（paste.rs::write_file_list）：写前 `File::open` 触发分目录 TCC 授权弹窗 + 写后校验 `pasteboardItems.count` + 被拦时系统通知建议 FDA + 文本降级。用户即时解法：给 ClipMan **完全磁盘访问权限**（现装构建即可生效）；重建后无 FDA 也可用（首次粘贴弹一次目录授权；他 App 容器内文件仍需 FDA）。另：objc2 直写替代 arboard（绕开 canonicalize）、`tauri-plugin-single-instance` 杜绝新旧实例混跑，均已合入
   - **incremental_vacuum 从未生效的 bug**（用户日志暴露）：该 PRAGMA 每释放一批页会返回一行，`execute()` 见到行就报错中断；已改为 prepare+query+drain 全量回收
   - **深色模式阴影**：已移除全部 CSS 阴影（半透明像素会污染窗口 alpha 形状，原生阴影按其外轮廓画出灰色弧带）+ 面板改为填满窗口（无透明边距）
   - **权限横幅**：重设计为设计令牌驱动的紧凑卡片（原琥珀色大横幅被用户否决）
2. 其余首轮验证项：浏览器富文本徽标+⌥Enter、⌘Click 多选合并粘贴、托盘暂停采集、设置页新项与"重置"
3. **README.md / README_EN.md 更新**（发版时）：版本徽章还是 2.0.1；缺 6-7 个已上线特性（文件、富文本、⌥Enter、多选合并、秘密检测、应用忽略、暂停采集）——审计详情见 git 历史中本条的来源会话
4. **提交**：验证通过后按功能拆分提交序列（用户明确要求后才 commit）
5. Wave 4 候选（未排期）：Paste Stack 逐次粘贴、类型识别与语法高亮、SQLCipher 加密（同步前置）、搜索 1000 条截断提示、Apple 公证

## 双模型代码审核（2026-07-07，Fable×3 分域 + Codex GPT-5.5 xhigh 全库）

四路审毕、交叉去重约 55 条，无结构性缺陷。**完整可执行的修复规格已落盘：`docs/dev/REVIEW-2026-07-07.md`**（P1 正确性 16 条 / P2 清理 14 条 / P3 简化 21 条，每条含位置与修法、验证状态标记）。两个待用户决策：① `group_name` 删运行时字段还是留（README 路线图有分组计划）；② 短查询 4096 字节截断是否接受为设计取舍。修复波次未开始。

## 已知问题 / 注意事项

- 搜索结果静默截断在 1000 条（storage.rs，无 UI 提示）
- `ignored_apps` 按应用本地化名称匹配（非 bundle id），跨语言环境有局限（有意为之的 v1 取舍）
- 多选合并粘贴跳过图片项（v1 限制，有日志计数）
- 后端 `notify_copied` 仍有硬编码中文串（i18n 债务）
- 默认语言写死 zh-CN，不跟随系统

## 文档与自动化

- 文档体系与维护协议：见 `AGENTS.md`「Documentation map & maintenance protocol」（2026-07-07 建立）
- Hooks（`.claude/settings.json` + `.claude/hooks/`）：SessionStart 自动注入本文件；Stop 时若源码比本文件新则提醒更新（首次生效可能需要运行一次 `/hooks` 或重启会话）

## 背景资料指路

- 竞品分析与长期路线图：claude.ai artifact「ClipMan 盲点报告与路线图」（2026-07-07）
- v2.2 执行记录（波次、验收、偏差裁决）：`docs/dev/PLAN.md` + `SPEC-1..4`
- 6 月 v2.0 重设计的历史文档已归档：`docs/archive/`（带过期横幅，勿作当前指导）
