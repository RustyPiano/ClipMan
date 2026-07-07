# ClipMan v2.2 开发总体规划（多 Agent 协作）

> 状态：进行中 · 创建于 2026-07-07 · 协调者：Claude（主会话）负责规格、派工、审查与验收

## 背景与目标

基于 2026-07-07 的盲点审计与竞品调研（结论见 claude.ai artifact「ClipMan 盲点报告与路线图」），本轮开发目标：

1. **修复用户确认的三个核心缺陷**：
   - 同一次复制产生多条记录（文本+图片各存一条）
   - Finder 复制文件被错误采集为"图片 + 文字"两条记录
   - 没有富文本（RTF/HTML）支持
2. **补齐 P0 信任底座**：CI 测试关卡、启动崩溃恢复、锁一致性、数据库空间回收
3. 为后续波次（分页、应用忽略、秘密检测、粘贴队列）铺路

## 架构决策（已定，Agent 不得擅自更改）

| 决策 | 内容 | 理由 |
|---|---|---|
| D1 | **一次剪贴板变化 = 一条记录**。按优先级取单一主格式：`Files > Text(+html 附带) > Image` | 修复重复记录的根因；Finder 复制命中 Files 后不再看文本/图标；Excel 类"文本+HTML+单元格图"命中 Text 并附带 html |
| D2 | 富文本用 **HTML 表示**（`clips.html TEXT` 列，随 Text 主格式附带），不引入 RTF 解析 | HTML 是跨平台交换格式（macOS public.html / Windows CF_HTML），arboard 3.6 Get/Set 均支持；RTF 只有 macOS 原生应用用，后续可加 |
| D3 | 文件类型 `ContentType::Files`，content = 绝对路径按 `\n` 连接的 UTF-8 文本 | 可直接进 FTS 索引（按路径搜索）、预览零成本；arboard 3.6 `get()/set().file_list()` 三平台可用 |
| D4 | 数据库迁移走 `add_column_if_missing` 追加式（与 source_app 列做法一致），**不 bump user_version** | 纯追加列，旧版本降级打开也安全 |
| D5 | 自复制标记（CopyMarker）始终对**主格式的规范化内容**取哈希：Text→纯文本字节（无视 html）、Files→连接后的路径字节、Image→现有规范化 RGBA | 粘贴写回 html 时 alt 纯文本与标记一致，监控端 get_text 读到 alt 即可命中标记 |
| D6 | 去重（content_hash）继续按主内容哈希；重复命中时 `html = COALESCE(新html, 旧html)` | 重复复制刷新时间戳保留元数据（现有语义），富格式"最近一次富复制"生效 |

## 任务分解与波次（按文件冲突域划分，一个文件同一波只属于一个 Agent）

### Wave 1（并行，互不冲突）
| 编号 | 任务 | 规格 | 文件域 | Agent 模型 |
|---|---|---|---|---|
| W1-A | CI 测试关卡（任务#2） | SPEC-3 §1 | `.github/workflows/ci.yml`、`package.json` | Sonnet |
| W1-B | 采集管线重构 + Files 类型 + HTML 富文本（后端）（任务#4/#5/#6） | SPEC-1 | `src-tauri/src/{storage,clipboard,paste,commands,tray}.rs` | Opus |
| W1-C | settings.rs 改用 safe_lock（任务#14） | SPEC-3 §3 | `src-tauri/src/settings.rs` | Sonnet |

### Wave 2（W1-B 验收后启动）
| 编号 | 任务 | 规格 | 文件域 |
|---|---|---|---|
| W2-A | 前端：files/richText 展示 + ⌥Enter 纯文本粘贴（任务#10 前端） | SPEC-2 | `src/lib/**`、`src/routes/**` |
| W2-B | 启动崩溃恢复（任务#3） | SPEC-3 §2 | `src-tauri/src/main.rs`、`migration.rs`、`storage.rs`（`ClipStorage::new` 签名） |
| W2-C | VACUUM 空间回收（#9）+ 单条大小上限（#7 后端） | SPEC-3 §4/§5 | `src-tauri/src/{storage,clipboard,settings}.rs` |

### Wave 3（规格在 Wave 2 验收后撰写）
QuickBar 分页（#8）、应用忽略列表+暂停采集（#11）、秘密自动检测（#12）、粘贴队列/多选（#13）

## Agent 工作守则（每个派工 prompt 都会附带）

1. **环境**：`export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"`。Rust 工具链 1.96.0（rust-toolchain.toml 固定），bun 已装好，node_modules 已恢复。
2. **只改自己文件域内的文件**。发现需要改域外文件 → 停下来，在最终报告里说明，不要动手。
3. **工作区有未提交的 WIP**（source_app 来源追踪、深色模式阴影修复等）——这些是有效改动，**绝不能回退或覆盖**。改动前先读文件当前状态。
4. **不要 git commit / stash / checkout**。协调者统一处理版本控制。
5. 代码风格跟随现有代码：错误用 `Result<_, String>`、锁用 `crate::safe_lock`、日志用 `log::` 宏、注释密度与现状一致（只解释"为什么"，不解释"是什么"）。
6. **完成前必须本地验证**：
   - Rust：`cargo test`（在 src-tauri/ 下）全绿 + `cargo clippy --all-targets 2>&1 | tail` 无新增 warning
   - 前端：`bun test tests/` 全绿 + `bun run check` 无错误 + `bun run lint`
   - 注意：`cargo test` 需要 `dist/` 存在，若报 frontendDist 缺失先跑 `bun run build`
7. **最终报告必须包含**：改动文件清单、每项验收标准的达成情况、与规格的任何偏差及原因（"Deviations" 一节，哪怕为空也要写"无偏差"）。
8. 规格里的验收标准是合同：做不到就如实报告，不要静默降级。

## 审查与验收流程（协调者执行）

1. Agent 报告完成 → `git diff` 审查改动范围是否越域、风格是否一致、是否符合架构决策 D1-D6
2. 本地跑完整验证：`cargo test` + `cargo clippy` + `bun test` + `bun run check` + `bun run build`
3. 对照规格逐条核验收标准
4. 不合格 → 用 SendMessage 打回给同一个 agent 修改（保留其上下文）
5. 合格 → 更新任务清单，启动下一波

## 状态跟踪

| 任务 | 状态 |
|---|---|
| 深色模式矩形阴影修复 | ✅ 已完成（window.rs + app.css） |
| 本地工具链 | ✅ rustup 1.96.0 + bun + node_modules 就绪 |
| W1-A CI | ✅ 已验收；clippy 已验证零警告并收紧为 `-D warnings`；eslint globals 补 setInterval/clearInterval（协调者代改，域外遗留问题） |
| W1-B 采集管线后端 | ✅ 已验收：68 测试全绿、clippy -D warnings 零警告、fmt 干净。偏差 1 项已接受（concealed 检查保留在调用侧，功能等价） |
| W1-C settings 锁 | ✅ 已验收 |
| W2-A 前端 files/富文本 | ✅ 已验收：lint/check/test/build 全绿（33 测试，+3）。偏差 4 项均已接受（fetchFullClip 扩展到 files 为满足完整路径预览所必需；测试文件属任务要求；徽标位置因 line-clamp 调整）。三项手动验证待协调者跑 app 确认 |
| W2-B 启动崩溃恢复 | ✅ 已验收：72 测试全绿（+4 失败注入测试），降级链纯函数化可测。协调者审查时发现并补上一处加固：致命路径先销毁 webview 窗口，防止未 manage 的 AppState 被隐藏窗口的命令调用触发 panic-abort。偏差 5 项均接受（含 commands.rs:980 一行域外机械适配） |
| W2-C VACUUM + 大小上限 | ✅ 已验收：83 测试全绿（+11），auto_vacuum 一次性迁移 + 事务后 reclaim_space，大小判定抽纯函数可测。偏差 2 项均接受 |
| W3-A QuickBar 键集分页 | ✅ 已验收：105 Rust + 38 前端测试全绿；键集游标 (timestamp,id) 决胜、查询计划无临时排序、重复时间戳翻页专测。偏差 4 项均接受（索引改名幂等迁移、prune 保持不变、零 IPC 重置切片、前端测试文件属规格要求） |
| W3-B 秘密自动检测 | ✅ 已验收：8 类高置信模式 + 20 正负例测试；skip_secrets 默认开、可关；regex 依赖为已有传递依赖提升为直接依赖（1.12.3，零新包） |
| W3-C 应用忽略 + 暂停 + 设置 UI | ✅ 已验收。它标记的域外遗留 bug（设置页 Reset 字面量缺新字段 → update_settings 反序列化失败）由协调者修复：Settings 加 `#[serde(rename_all, default)]`、types.ts 字段转必填 + capturePaused、+page.svelte 两处字面量补齐 |
| W3-D 多选合并粘贴 | ✅ 已验收：合并保序、Image 跳过计数、全图选择保护剪贴板；paste_clip 原路径零改动 |
| **终验** | ✅ Rust 117 测试 / clippy `-D warnings` / fmt；前端 42 测试 / lint / check / build 全绿；i18n 并行追加零冲突（9 新键 × 3 处齐全）；总 diff 24 文件 +3031/-261 |
