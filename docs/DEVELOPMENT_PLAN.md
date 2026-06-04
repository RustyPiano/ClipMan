# ClipMan v2.0.0 开发计划（并行友好 · v2 收紧版）

> 配套规格：[`REDESIGN_SPEC.md`](./REDESIGN_SPEC.md)。本文是**给执行 Agent 看的施工图**：每个工作包（WP）标了「负责文件 / 依赖 / 并行组 / 步骤 / 验收标准」。
>
> **v2 修订要点（评审后收紧的硬约束）**：① Windows 焦点模型先 spike，不用 `WS_EX_NOACTIVATE`；② FTS5 表结构写死；③ 自捕获标记是跨文件契约，提到 Phase 0；④ 图片改「始终存原图 + 缩略图列」；⑤ 最近/常用数据契约拆清；⑥ 命令桩用 `Err("not implemented")`，禁止 `todo!()`。
>
> **当前状态（2026-06-04）**：Phase 0 的数据层/设置/命令契约和 Phase 1 的 QuickBar、自动粘贴、前端交互、隐私/图片捕获已有实现；不要把本文当成“全部未开始”的清单。本文保留为实施回顾与后续收尾清单，已落地项以当前代码和本段状态说明为准。macOS 平台焦点与自动粘贴链路已验证通过，Windows 仍需按 Phase 3 真机矩阵验证后才能宣称完成。
>
> **并行原则**：并行的 WP 各自拥有不重叠文件；共享文件（`main.rs`/`commands.rs`/`settings.rs`/`Cargo.toml`）的改动集中在 Phase 0 打桩，Phase 1 之后只在各自函数/区域追加。

---

## 0. 全局约定

- **不破坏可编译性**：每个 WP 结束时 `cargo build` 与 `bun run build` 都通过。
- **桩用 `Err`，不用 `todo!()`**：新命令在 Phase 0 注册签名，桩体一律 `return Err("not implemented".into())` 或最小 no-op，**保证 app 运行期不 panic**（前端并行接命令时不会崩）。
- **平台目标**：macOS + Windows 一流，Linux 降级不报错。`#[cfg(target_os=...)]` 两端都要给。
- **提交粒度**：每个 WP 一组提交，conventional commits 前缀（如 `feat(quickbar):`）。
- **测试**：Rust `#[cfg(test)]` 单测；端到端在 Phase 3 过测试矩阵。

### 文件归属总表（避免并行打架）

| 文件 | 主要归属 WP |
|---|---|
| `src-tauri/src/storage.rs` | WP-0.1 / 0.2（数据层，单一负责） |
| `src-tauri/src/crypto.rs` | WP-0.1（删除） |
| `src-tauri/src/migration.rs` | WP-0.1 |
| `src-tauri/src/settings.rs` | WP-0.3 |
| `src-tauri/src/commands.rs` | WP-0.4 建桩；WP-1.B / 2.A 填各自函数体 |
| `src-tauri/src/main.rs` | WP-0.1（删密钥 + 改 AppState/marker）→ WP-1.A（窗口/快捷键），**串行** |
| `src-tauri/src/window.rs`（新增） | WP-1.A |
| `src-tauri/src/paste.rs`（新增） | WP-1.B |
| `src-tauri/src/clipboard.rs` | WP-1.D |
| `src-tauri/src/tray.rs` | WP-1.A（设置窗口入口）；缩略图读取 |
| `src/lib/stores/*.svelte.ts` | WP-1.C |
| `src/routes/+page.svelte`、`ClipboardItem.svelte`、`SearchBar.svelte` | WP-1.C |
| `src/lib/components/pinned/*`、标签编辑 | WP-2.A |
| `src/routes/settings/*` | WP-2.B |
| `src/lib/types.ts` | WP-0.4 |
| `CopyMarker` 类型定义 | WP-0.1（与 AppState 同处，`storage.rs` 或 `main.rs`） |

---

## 1. 依赖图与并行组

```
Phase 0  基础/契约（先行）
  ├─ WP-0.0 Spikes(Windows 焦点模型 / macOS 面板)   ← 最先验证，结论回填 1.A/1.B
  ├─ WP-0.1 数据层重置(删加密+schema(+thumbnail/label)+迁移+WAL+CopyMarker+AppState)  ★关键路径
  ├─ WP-0.2 FTS5(写死表结构 + Rust 维护 + <3 回退)   (依赖 0.1)
  ├─ WP-0.3 设置字段(+废弃 store_original_image)     (独立，可与 0.1 并行)
  └─ WP-0.4 命令契约(拆 recent/pinned + marker 语义 + 桩用 Err) + TS 类型  (依赖 0.1/0.3)
            │
            ▼
Phase 1  并行 fan-out
  ├─[流A 原生] WP-1.A QuickBar 窗口(mac 面板 / win 记录恢复 / Settings 窗口)
  ├─[流B 原生] WP-1.B 自动粘贴(enigo + 平台焦点 + 设置 CopyMarker)
  ├─[流C 前端] WP-1.C QuickBar 交互(键盘/选中/双面板/后端单源)
  └─[流D 后端] WP-1.D 隐私(跳过密码类) + 检查 CopyMarker + 图片捕获存原图&缩略图
            │
            ▼
Phase 2  ├─ WP-2.A 常用 UX(标签/排序/槽位)   ‖   WP-2.B Settings 窗口 UI
         └─ WP-2.C 第二快捷键直达常用(可选)
            │
            ▼
Phase 3  WP-3.A 集成 + 平台测试矩阵（串行收尾）
```

**可并行组：**
- P0：`WP-0.0`（spike，尽早）与 `WP-0.1` 可并行（spike 不写生产代码）。`WP-0.3` 与 `0.1` 并行；`0.2` 依赖 `0.1`；`0.4` 依赖 `0.1/0.3`。
- P1（最大并行）：`WP-1.A`、`WP-1.C`、`WP-1.D` **文件完全不重叠，可同时下发**。`WP-1.B` 逻辑放 `paste.rs`、只在 `commands.rs` 填已建好的函数体，则与 `1.A` 并行（1.A 不碰 commands.rs）。
- P2：`WP-2.A`、`WP-2.B` 并行；`WP-2.C` 收尾。

---

## Phase 0 — 基础与契约

### WP-0.0 前置 Spikes（最先验证，避免 Phase 1 返工）
- **目标**：用最小代码验证两条高风险路径，把结论写回 1.A/1.B。**不进生产代码**，产出是一段验证记录 + 选定方案。
- **负责文件**：临时分支/示例，不动主代码。
- **依赖**：无。
- **S1 — Windows 焦点模型**（高风险）：验证「`GetForegroundWindow` 记录原窗口 → QuickBar 正常取焦点供输入 → 隐藏 → `SetForegroundWindow(原窗口)`（必要时 `AttachThreadInput`）→ `enigo` 发 `Ctrl+V`」能稳定把内容粘进原应用。**确认不使用 `WS_EX_NOACTIVATE`**（其顶层窗口不会成前台，与搜索框输入冲突）。测 3 个目标：记事本、浏览器地址栏、VS Code。
- **S2 — macOS 面板**：验证 Tauri v2 窗口句柄配合 `NSWindowStyleMaskNonactivatingPanel` 的 non-activating 面板**能成为 key window 接收搜索框输入**，同时 app 不变前台（菜单栏不切到 ClipMan），`Cmd+V` 能粘回原应用。`tauri-nspanel` 曾是候选方案，当前实现未采用。
- **验收**：两条路径各有一份「可行/坑点/最终选定 API」记录；若 S1 的「恢复前台」在某些应用失败，记录降级策略（如仅复制提示）。
- **产出回填**：更新 WP-1.A/1.B 的实现细节。

### WP-0.1 数据层重置（删加密 + schema + 迁移 + WAL + CopyMarker）★关键路径
- **目标**：移除加密层；升级表结构（含 `thumbnail`/`label`/`group_name`）；一次性迁移；开 WAL；定义跨文件的 `CopyMarker` 并改 `AppState`。
- **负责文件**：`storage.rs`、`crypto.rs`(删)、`migration.rs`、`main.rs`(删密钥初始化 + 改 AppState)、`Cargo.toml`(删 `ring`)。
- **依赖**：无。**最先做**（独占 main.rs/storage.rs）。
- **步骤**：
  1. 删 `crypto.rs`；`main.rs` 删 `get_or_create_encryption_key`、`Crypto`、相关 `use`。
  2. `ClipStorage` 去 `crypto` 字段；`insert`/各 `get_*`/`search` 直接存取明文 BLOB。
  3. 建表/迁移加列：`thumbnail BLOB`、`label TEXT`、`group_name TEXT`（参考现有 content_hash 的 `ALTER TABLE ADD COLUMN` 写法做存量库迁移）。保留 `is_pinned`/`pin_order`/`content_hash` 及现有索引。
  4. 打开连接后 `PRAGMA journal_mode=WAL;`。
  5. **拆数据契约**：新增 `get_recent_clips(limit)`（**WHERE is_pinned=0**，按 timestamp DESC）、`get_pinned_clips()`（按 pin_order ASC）。保留 `get_by_id`。旧 `get_recent`(混排) 标注弃用或删除。
  6. **定义 `CopyMarker`**：`struct CopyMarker { hash: String, content_type: ContentType }`（放 `storage.rs`）。`hash` 是**规范化剪贴板 payload** 的 SHA256，不等同于 DB `content_hash`：文本用 UTF-8 bytes；图片用写入/读回两侧共用的规范化表示（如 `width + height + RGBA bytes`），避免 PNG 重编码导致 marker 失效。`AppState.last_copied_by_us` 类型从 `Arc<Mutex<Option<String>>>` 改为 `Arc<Mutex<Option<CopyMarker>>>`。**只改类型与 AppState 字段，set/check 的具体逻辑分别由 1.B/1.D 实现**——本 WP 仅提供稳定类型，避免它们各改一半。
  7. 迁移（`migration.rs`）：`PRAGMA user_version<1` 时——存在 `.clipman.key` 则逐行解密→明文重写（失败行删）、删密钥；设 `user_version=1`。整体失败允许清空 clips 兜底。
  8. 改/删相关单测（crypto 测试删；存取测试改明文）。
- **验收**：`cargo build` 通过；全仓无 `Crypto`/`ring`/写 `.clipman.key`；新条目明文；旧加密库能读出或干净重置；`get_recent_clips` 不含置顶项；`CopyMarker` 类型与 AppState 已就位。
- **注意**：独占 main.rs，**必须在 WP-1.A 之前完成**。

### WP-0.2 FTS5 搜索（写死表结构）
- **目标**：真·全文索引，替换内存子串匹配。
- **负责文件**：`storage.rs`、`migration.rs`(建表/回填)。
- **依赖**：WP-0.1。
- **步骤**：
  1. 建表（**照抄，不要改结构**）：`CREATE VIRTUAL TABLE clips_fts USING fts5(clip_id UNINDEXED, search_text, label, tokenize='trigram');`，约定 `clips_fts.rowid = clips.rowid`。
  2. **Rust 侧维护**（不用 SQL 触发器）：在 `insert`/`delete`/`set_clip_label`/置顶相关方法里同步写删 FTS 行——Rust 手里有解码后的纯文本与 label，避免对 BLOB 在 SQL 里 `CAST`。`search_text` 当前仅文本类型填充（图片/二进制留空）；若将来支持文件条目，再扩展为文件名。
  3. **查询**：`query.chars().count() >= 3` 走 `MATCH`（转义用户输入）；`< 3`（含中文一两字）回退 `LIKE '%q%'` 扫描。命中后用 rowid→`clips` 取完整条目，返回带 `is_pinned`。
  4. 迁移时用 Rust 手动回填：清空 `clips_fts`，遍历 `clips`，解码得到 `search_text`，再 `INSERT INTO clips_fts(rowid, clip_id, search_text, label) VALUES (clips.rowid, id, search_text, label)`。不要依赖 `INSERT INTO clips_fts(clips_fts) VALUES('rebuild')`，因为当前设计不是 external-content FTS 表。
- **验收**：中/英文子串、单字中文、label 都能搜到；空查询回退全量；大数据下明显快于旧实现。
- **注意**：确认 `rusqlite` 启用 `bundled` 且 SQLite ≥3.34（trigram 支持）。

### WP-0.3 设置字段
- **目标**：增 3 字段、废弃 1 字段。
- **负责文件**：`settings.rs`。
- **依赖**：无（可与 0.1 并行）。
- **步骤**：`Settings` 加 `auto_paste: bool=true`、`ignore_concealed: bool=true`、`pinned_shortcut: Option<String>=None`；**移除 `store_original_image`**（图片改为始终存原图，见 1.D）。`Default`/`load`/`save` 三处同步；`load` 对缺字段取默认、对已废弃字段忽略，不报错。
- **验收**：设置读写往返；旧 `settings.json`（含 `store_original_image` 或缺新字段）加载不报错。

### WP-0.4 命令契约 + 共享类型（建桩，桩用 Err）
- **目标**：一次定好本期所有 Tauri 命令签名并注册，后续只填实现。
- **负责文件**：`commands.rs`、`main.rs`(invoke_handler，**在 WP-0.1 之后**)、`src/lib/types.ts`。
- **依赖**：WP-0.1（类型/新查询）、WP-0.3（Settings）。
- **步骤**：
  1. 注册命令（桩体 `Err("not implemented".into())` 或最小实现，**禁用 `todo!()`**）：
     - 查询：`get_recent_clips(limit)`、`get_pinned_clips()`、`search_clips(query)`（返回带 `isPinned` 的列表）。
     - 取用：`paste_clip(id, mode: "paste"|"copy")` —— 统一入口。
     - 常用：`set_clip_label(id, label: Option<String>)`、`reorder_pinned(id, direction: "up"|"down")`、`toggle_pin(id, isPinned)`。
     - 窗口：`open_settings_window()`、`hide_quickbar()`。
     - 保留：`delete_clip`、`get_settings`、`update_settings`、`clear_*`、更新检查等。
  2. **CopyMarker 语义写进本 WP 注释/文档**：谁设（1.B 写剪贴板时，`hash=SHA256(normalized_clipboard_payload)`、带 content_type）、谁查（1.D 收到剪贴板变化时按同一规范化规则算 hash 比对、命中跳过）、何时清（写后约 2s 清，沿用现机制）。
  3. `FrontendClipItem`/`ClipItem` 加 `label`；图片的 `content` 字段改为**缩略图 data URL**（原图按需取，用于粘贴）；`types.ts` 同步 `label?: string`、`isPinned` 等。
  4. 全部加入 `invoke_handler![]`。
- **验收**：前后端类型一致；`bun run build` + `cargo build` 通过；前端 `invoke` 任意命令返回明确「未实现」而非 panic。

---

## Phase 2 之前的并行 — Phase 1

### WP-1.A QuickBar 窗口（mac 面板 / win 记录恢复 / Settings 窗口）【流A】
- **目标**：main 窗口改造成不抢焦点弹窗；新建 Settings 普通窗口。
- **负责文件**：`tauri.conf.json`、`main.rs`(窗口/快捷键段)、`window.rs`(新增)、`tray.rs`(设置入口)。
- **依赖**：WP-0.0（spike 结论）、WP-0.1（main.rs 已清理）、WP-0.3（快捷键设置）。
- **步骤**：
  1. `tauri.conf.json`：main → `decorations:false`、`alwaysOnTop:true`、`skipTaskbar:true`、`visible:false`、`resizable:false`、自适应尺寸（最大 820×600）。新增 `settings` 窗口（普通、有边框、`visible:false`）。
  2. **macOS**：通过 Tauri 窗口句柄直接设置 `NSWindowStyleMaskNonactivatingPanel`、floating level 与 collection behavior，封装进 `window.rs::setup_quickbar_macos()`；`tauri-nspanel` 只保留为后续兼容性备选。
  3. **Windows**（按 S1 结论）：main 为**正常可取焦点**窗口（`WS_EX_TOOLWINDOW` 不进任务栏，**不加 `WS_EX_NOACTIVATE`**）。在唤起前用 `GetForegroundWindow()` 记录原前台窗口（存到一个 `Arc<Mutex<Option<HWND-ish>>>` 状态，供 1.B 取用恢复）。封装 `window.rs::setup_quickbar_windows()` + `remember_foreground()`。
  4. 唤起：全局快捷键 → 活动屏幕中心略偏上定位 → show + 聚焦搜索；macOS 面板成 key、app 不前台；Windows 正常取焦点（原窗口已记录）。
  5. 失焦自动隐藏：监听 blur → `hide`。
  6. `open_settings_window()`：show/focus settings 普通窗口；托盘「设置」改调它。`hide_quickbar()` 实现。
- **验收**：快捷键弹出居中无边框窗；macOS 下原应用仍前台（菜单栏不变）；Windows 下记录了原前台窗口；点别处/Esc 关窗；托盘「设置」打开独立有边框窗口。
- **注意**：本 WP 负责窗口/快捷键/记录原前台；**粘贴逻辑不在此**（在 1.B）。不碰 commands.rs。

### WP-1.B 自动粘贴引擎（含设置 CopyMarker）【流B】
- **目标**：取用时写剪贴板 + 按平台模拟粘贴回原应用。
- **负责文件**：`paste.rs`(新增)、`commands.rs`(填 `paste_clip` 体)、`Cargo.toml`(加 `enigo`)。
- **依赖**：WP-0.0(S1)、WP-0.4（`paste_clip` 桩 + 类型）、概念上配合 WP-1.A 的窗口/原前台记录。
- **步骤**：
  1. `paste.rs::simulate_paste()`：macOS 发 `Cmd+V`；Windows 先 `SetForegroundWindow(原前台)`（取 1.A 记录的句柄，必要时 `AttachThreadInput`）再发 `Ctrl+V`；Linux 检测 Wayland/失败返回「未粘贴」。
  2. `paste_clip(id, mode)`：取条目（**图片取原图 `content`，非缩略图**）→ 写系统剪贴板，**同时设置 `CopyMarker{hash=SHA256(normalized_clipboard_payload), content_type}`**（文本/图片统一；图片 hash 用与监控侧相同的规范化 RGBA/尺寸表示，不用 DB PNG 字节）→ 若 `mode=="paste"` 且设置 `auto_paste` 开 → `hide_quickbar` → `simulate_paste()`；`mode=="copy"` 只复制。写后约 2s 清 marker（沿用现机制）。
  3. macOS 辅助功能未授权 → 返回可识别错误码，前端提示。
- **验收**：`paste` 模式内容直接出现在原应用光标处、不产生自捕获重复项；`copy` 模式只进剪贴板；Windows 能恢复原前台再粘；macOS 未授权有提示。
- **注意**：marker 的**设置**在此；**检查**在 1.D。二者共用 0.1 定义的 `CopyMarker`。

### WP-1.C QuickBar 交互 + 后端单一数据源【流C】
- **目标**：键盘驱动、选中态、整卡可点、双面板、前端状态重构。
- **负责文件**：`+page.svelte`、`ClipboardItem.svelte`、`SearchBar.svelte`、`clipboard.svelte.ts`、新增 `selection.svelte.ts`。
- **依赖**：WP-0.4（类型/命令）。与原生流并行。
- **步骤**：
  1. **后端单一数据源重构**：`clipboard-changed`/`history-cleared` 到来时以后端查询结果为准（最近用 `get_recent_clips`、常用用 `get_pinned_clips`、搜索用 `search_clips`）。当前 store 仍保留轻量 incoming 合并/replay，用于降低后台事件丢失和窗口唤起延迟；禁止在前端复制后端的淘汰、排序、搜索规则。
  2. `selection.svelte.ts`：`selectedIndex` + 当前面板（recent/pinned）；`↑↓` 移动 + 自动滚动；鼠标悬停同步选中。
  3. 窗口级 keydown：`↑↓`、`Enter`→`paste_clip(default)`、`Cmd/Ctrl+Enter`→相反 mode、`Cmd/Ctrl+1-9`→取第 N（普通数字保留给搜索框输入）、`Tab`→切面板、`Esc`→`hide_quickbar`、`Cmd/Ctrl+P`→`toggle_pin`、`Cmd/Ctrl+Delete`→`delete_clip`。
  4. `ClipboardItem`：整卡 `onclick`=取用；保留悬停的置顶/删除/编辑按钮；选中高亮 + 左侧 1~9 序号。
  5. 双面板：`Tab` 切「最近/常用」，常用读 `get_pinned_clips`。
  6. 搜索框打开自动聚焦；任意打字聚焦搜索。
- **验收**：全键盘可完成「打开→过滤→选中→取用」；卡片单击即取用；`Tab` 切面板；选中高亮滚动跟随；前端不复制后端淘汰/排序规则。
- **注意**：真实粘贴效果依赖 1.B，行为联调放 Phase 3。

### WP-1.D 隐私（跳过密码类）+ 检查 CopyMarker + 图片捕获策略【流D】
- **目标**：复制密码不入历史；用哈希 marker 防自捕获；图片捕获改「存原图 + 缩略图」。
- **负责文件**：`clipboard.rs`。
- **依赖**：WP-0.1（`CopyMarker`、`thumbnail` 列）、WP-0.3（`ignore_concealed`，已移除 `store_original_image`）。
- **步骤**：
  1. **检查 CopyMarker**：把现有「文本字符串相等」自捕获判断，改为对来料内容按统一规则规范化后算 `SHA256`，再与 `last_copied_by_us` 的 `CopyMarker.hash` 比对，命中则跳过（文本/图片统一）。
  2. **跳过密码类**（`ignore_concealed` 开时）：macOS 读 `NSPasteboard` 类型，含 `org.nspasteboard.ConcealedType`/`TransientType`/`AutoGeneratedType` 则不保存；Windows 检测 `ExcludeClipboardContentFromMonitorProcessing`/`CanIncludeInClipboardHistory` 命中则跳过。
  3. **图片捕获策略**：删除 `store_original_image` 分支；统一**存原图（2048 上限）到 `content` + 存 256px 缩略图到 `thumbnail`**（复用 `process_full_image` + `create_thumbnail` 两段，结果分别落两列）。
- **验收**：从密码管理器复制密码不进历史；自动粘贴的文本/图片不产生重复项；新捕获图片在 DB 里 `content`=原图、`thumbnail`=缩略图。
- **注意**：tray 渲染改读 `thumbnail`（tray.rs 由 1.A 负责协调；若 1.A 未动到，列入 3.A 收尾）。

---

## Phase 2 — 功能补全

### WP-2.A 常用 UX（标签 / 排序 / 数字槽位）
- **目标**：常用面板命名、排序、稳定槽位落地。
- **负责文件**：`src/lib/components/pinned/*`(新增)、`ClipboardItem.svelte`(标签展示/编辑入口)、`clipboard.svelte.ts`(接 `set_clip_label`/`reorder_pinned`)。
- **依赖**：WP-0.4（命令）、WP-1.C（交互框架）。
- **步骤**：
  1. 常用项显示 `label`（无则显示内容预览）；卡片「编辑标签」入口→内联输入→`set_clip_label`。
  2. 常用面板 `Cmd/Ctrl+Shift+↑/↓`→`reorder_pinned`；（可选）拖拽排序。
  3. 数字槽位 1~9 稳定对应 `pin_order`。
- **验收**：起名后显示名称且可搜；调序持久、槽位稳定；`Tab`→按 `2` 恒定粘同一条。

### WP-2.B Settings 窗口 UI
- **目标**：独立窗口管理所有设置（含新项）。
- **负责文件**：`src/routes/settings/*`、`src/lib/components/settings/*`。
- **依赖**：WP-0.3（字段）、WP-1.A（settings 窗口存在）。
- **步骤**：加「自动粘贴 `auto_paste`」「忽略密码类 `ignore_concealed`」「常用第二快捷键 `pinned_shortcut`(可选录入)」；**移除 `store_original_image` 及任何加密相关 UI**；数据管理（清空、存储位置）归此窗口。
- **验收**：改设置即时生效并持久；托盘与弹窗齿轮都能打开此窗口。

### WP-2.C 第二全局快捷键直达「常用」（可选）
- **目标**：`pinned_shortcut` 唤起 QuickBar 并直接进常用面板。
- **负责文件**：`main.rs`/`commands.rs`(注册/更新)、前端(接收「以常用面板打开」参数)。
- **依赖**：WP-1.A、WP-1.C。
- **步骤**：设了 `pinned_shortcut` 就注册，触发时 show + 通知前端切常用；改设置时同步重注册（参考现 `global_shortcut` 重注册 `commands.rs:432-455`）。
- **验收**：设了即一按直达常用；清空则解绑。

---

## Phase 3 — 集成与平台 QA

### WP-3.A 端到端联调 + 测试矩阵（串行收尾）
- **目标**：接通各流，过平台测试。
- **依赖**：全部。
- **测试矩阵**（macOS + Windows 各跑）：
  - 目标应用：VS Code、浏览器地址栏/输入框、Word/Pages、系统终端、微信/飞书输入框。
  - 用例：文本自动粘贴；图片自动粘贴（原图质量）；`copy` 模式不自动粘；`Tab`+数字取常用；常用起名/调序持久；失焦/Esc 关窗；macOS 辅助功能未授权提示；Windows 恢复原前台再粘；密码类不入历史；旧库迁移/重置；搜索（中/英/单字/标签/<3 回退）；tray 用缩略图。
- **当前验证状态**：macOS 已验证通过；Windows 与 Linux 降级路径仍需补齐记录。
- **验收**：剩余矩阵全绿；README 路线图勾选与实际一致（FTS5、自动粘贴等）；更新 README/release notes。
- **注意**：Linux 仅验证「降级为仅复制」不报错。

---

## 附录：建议执行顺序（给调度者）

1. **并行起步**：`WP-0.0`(spike) 与 `WP-0.1`(数据层，独占 main.rs/storage) 同开；`WP-0.3` 也可并行。
2. `WP-0.1` 完成后跑 `WP-0.2`(FTS)；`WP-0.4` 在 0.1/0.3 类型定后建桩。
3. **Phase 1 并行**：`WP-1.A`(独占 main.rs 窗口段)、`WP-1.C`(独占前端)、`WP-1.D`(独占 clipboard.rs) 同时开；`WP-1.B` 把逻辑放 `paste.rs`、只在 commands.rs 填函数体，可与 1.A 并行（1.A 不碰 commands.rs）。
4. **Phase 2 并行**：`WP-2.A`、`WP-2.B`；随后 `WP-2.C`。
5. `WP-3.A` 串行收尾。

> 用子 Agent 并行时，给每个 Agent 只授权其「负责文件」清单内的写权限，可最大程度避免互相覆盖。共享文件（main.rs/commands.rs）的 WP 不同时下发；`WP-0.0` 的 spike 结论务必回填 `WP-1.A/1.B` 后再开 Phase 1。
