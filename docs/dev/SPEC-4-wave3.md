# SPEC-4：Wave 3（分页 / 秘密检测 / 应用忽略 / 多选粘贴）

> 前置阅读 docs/dev/PLAN.md。基线（Wave 2 收官时）：Rust 83 测试全绿、clippy `-D warnings` 零警告、前端 33 测试 + lint/check/build 全绿。
> 派工批次：**批次 1**（并行）= §1 W3-A + §2 W3-B；**批次 2**（批次 1 验收后，并行）= §3 W3-C + §4 W3-D。批次划分即文件冲突域划分，不得越域。

## §1 QuickBar 键集分页（W3-A · Opus · 任务#8）

文件域：`src-tauri/src/storage.rs`、`src-tauri/src/commands.rs`、`src/lib/stores/clipboard.svelte.ts`、`src/routes/+page.svelte`（以及必要的 `src/lib/types.ts` 增量）

### 现状
`clipboard.svelte.ts` 每次打开面板 `get_recent_clips(limit: maxHistoryItems)`（可到 10000）→ 整个历史（含 base64 缩略图）一次性过 IPC。这是 Maccy #384（内存随历史增长）的病根，也是竞品调研中排名第 2 的差异化机会。

### 后端
1. storage.rs：新增键集分页查询
```rust
pub fn get_recent_clip_previews_page(
    &self, limit: usize, before: Option<(i64, &str)>,  // (timestamp, id) 游标
) -> Result<Vec<ClipPreviewItem>>
// WHERE is_pinned = 0 AND (?cursor 为空 OR timestamp < ?t OR (timestamp = ?t AND id < ?id))
// ORDER BY timestamp DESC, id DESC LIMIT ?limit
```
   - **排序必须补 `id DESC` 决胜键**（现有查询只按 timestamp，重复时间戳下键集游标会丢行/重行）；现有 `get_recent_clip_previews` 一并补齐保持一致
   - 查询计划测试：照抄现有 `*_uses_index_without_temp_sort` 测试模式验证不产生临时排序（必要时扩展 `idx_recent_unpinned_timestamp` 索引为 `(is_pinned, timestamp DESC, id DESC)`）
2. commands.rs：`get_recent_clips` 追加可选参数 `before_timestamp: Option<i64>, before_id: Option<String>`（都缺省 = 第一页，旧前端兼容）
3. 搜索（本波不分页，保持 1000 上限）与置顶列表（量小，全量）不动

### 前端
1. store：`recentItems` 改为按页累积；`loadMoreRecent()` 带 `isLoadingMore` 防抖与 `hasMoreRecent`（末页判定：返回条数 < PAGE_SIZE）；`PAGE_SIZE = 100`
2. 重置时机：`quickbar-opened` 刷新、历史清空事件、面板切换回 recent——回到第一页（沿用现有刷新路径，先读懂再改）
3. 触发加载：列表滚动到距底部 < 2 屏时；键盘向下选择越过已加载末尾时
4. `clipboard-changed` 事件的插入/去重逻辑保持现有语义（新项置顶；已存在项时间戳刷新要处理好它可能在任意已加载页中——先读现有实现，保持一致）
5. 游标来源：已加载最后一项的 `timestamp` + `id`（FrontendClipItem 两者都有）

### 验收
- [ ] 打开面板只拉第一页（100 条）；滚动/键盘到底自动续页；搜索/置顶行为不变
- [ ] 重复时间戳下翻页不丢行不重行（专门测试：同一 timestamp 插入 3 条，PAGE_SIZE=2 翻页取全）
- [ ] 旧调用（无游标参数）行为不变；Rust 与前端全套门禁绿（含查询计划测试）

## §2 秘密自动检测（W3-B · Sonnet · 任务#12）

文件域：`src-tauri/src/secrets.rs`（新建）、`src-tauri/src/clipboard.rs`（单一挂钩点）、`src-tauri/src/settings.rs`（一个字段）、`src-tauri/src/main.rs`（仅 `mod secrets;` 一行）

### 设计
1. 新模块 `secrets.rs`：
```rust
/// 返回命中的秘密种类（用于日志），None = 不是秘密
pub fn detect_secret(text: &str) -> Option<&'static str>
```
   高置信模式（v1 只做这些，**不做**熵启发式——误报会让用户困惑"为什么没记录"）：
   - PEM 私钥块：`-----BEGIN (RSA |EC |OPENSSH |ENCRYPTED |)PRIVATE KEY-----`
   - AWS Access Key ID：`\bAKIA[0-9A-Z]{16}\b`；AWS 临时密钥 `\bASIA[0-9A-Z]{16}\b`
   - JWT：`\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b`
   - GitHub token：`\b(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36,}\b`、`\bgithub_pat_[A-Za-z0-9_]{60,}\b`
   - Slack token：`\bxox[baprs]-[A-Za-z0-9-]{10,}\b`
   - OpenAI/Anthropic 风格：`\bsk-[A-Za-z0-9_-]{20,}\b`
   - 私钥助记词**不做**（自然语言误报高）
   - 用 `regex` crate（查 Cargo.toml 是否已有；没有则加 `regex = "1"`——这是唯一允许的 Cargo.toml 改动，报告中说明）；正则用 `std::sync::LazyLock` 编译一次
2. settings.rs：`skip_secrets: bool` 默认 `true`（normalize/默认值模式照抄 ignore_concealed）
3. clipboard.rs 挂钩：`process_text_change` 开头（大小检查之后）——`skip_secrets` 开且 `detect_secret` 命中 → `log::info!("🔒 Skipping captured secret ({kind})")` 后 return。只作用于 Text；Files/Image 不检测
4. 前端设置开关不在本任务（W3-C 统一做）

### 测试（secrets.rs 内）
每个模式一正一负；负例必须含：普通中文文本、URL、长 SHA-256 十六进制、base64 图片片段、"eyJ" 开头但只有两段的字符串。

### 验收
- [ ] 全部模式测试通过；`skip_secrets=false` 时不拦截（测试覆盖）
- [ ] Rust 全套门禁绿；Cargo.toml 若加 regex 在 Deviations 中说明版本

## §3 应用忽略列表 + 暂停采集 + 设置 UI（W3-C · Sonnet · 批次 2 · 任务#11 + #7/#12 的设置界面）

文件域：`src-tauri/src/{clipboard,settings,tray,main}.rs`、`src/lib/components/settings/**`、`src/lib/i18n/index.svelte.ts`、`src/lib/types.ts`（Settings 接口）

1. settings.rs：`ignored_apps: Vec<String>`（去重、trim、去空；上限 100 条）、`capture_paused: bool`（默认 false，持久化）
2. clipboard.rs：`handle_clipboard_event` 早期检查：`capture_paused` → 直接 return（在 concealed 检查旁）；对 Text/Files/Image 分发前取一次 `frontmost_app_name()`（避免与现有 process 函数里的重复 NSWorkspace 调用——重构为取一次传下去），若（大小写不敏感、trim 后）命中 `ignored_apps` → log 后 return。注意：跳过同样不得打断 last_marker 前移（沿用 record_marker_and_decide_dispatch 之后的位置或说明取舍）
3. tray.rs + main.rs：托盘加"暂停采集"CheckMenuItem（i18n 沿用 TrayI18n 模式），main.rs 菜单事件处理翻转设置并保存 + 刷新托盘
4. 前端设置页（components/settings/ 下的相应 tab，先读现状选正确落点）：
   - 忽略应用列表编辑器（文本输入添加 + 列表删除；说明文案注明按应用名匹配）
   - `skip_secrets` 开关（W3-B 的字段）
   - `max_text_bytes`（以 MB 显示）与 `max_image_dimension` 数值输入（W2-C 的字段）
   - Settings 接口/类型/i18n（zh+en）同步；**编辑 .svelte 前先加载 svelte-code-writer 技能**
5. 验收：全套 Rust+前端门禁绿；设置往返（保存→重载）测试；暂停时零采集、恢复后正常

## §4 多选合并粘贴（W3-D · Opus · 批次 2 · 任务#13）

文件域：`src-tauri/src/paste.rs`、`src-tauri/src/commands.rs`、`src/routes/+page.svelte`、`src/lib/stores/clipboard.svelte.ts`、`src/lib/components/ClipboardItem.svelte`、`src/lib/i18n/index.svelte.ts`

1. 后端：新命令 `paste_clips(ids: Vec<String>, mode: String, separator: String)`：
   - 按 ids 顺序取项；Text/Files 取纯文本（Files 用路径文本）；Image 项跳过并计数（v1 限制，返回值或日志说明）
   - 合并文本用 separator（前端只会传 `"\n"`/`"\t"`/`""`，后端不枚举校验，如实拼接）写回剪贴板（走现有 write 管线的 set_text + marker + TTL 清除），逐项 touch timestamp，然后按 mode 决定是否模拟粘贴（复用 paste_clip 的既有辅助函数，抽公共部分时保持 paste_clip 行为不变）
2. 前端：
   - ⌘Click（Windows Ctrl+Click）切换多选；选中态视觉（复选标记/边框，与现有选中态区分）；再次 ⌘Click 取消；Esc 先清多选再走原有 Esc 行为
   - 多选 ≥2 时底部提示变为"已选 N 项 · ↵ 合并粘贴（换行分隔）"；Enter 触发 `paste_clips`（separator 固定换行，后续再做可配置）；粘贴后清空多选
   - 多选状态存 store（`selectedIds: SvelteSet<string>` 或等价 runes 结构），面板切换/刷新时清空
   - **编辑 .svelte 前先加载 svelte-code-writer 技能**
3. 明确不做：Paste Stack 顺序逐次粘贴（后续独立任务）、分隔符设置项、拖拽多选
4. 验收：全套门禁绿；store 层测试覆盖 paste_clips 调用参数与顺序保持；后端测试覆盖合并顺序、Image 跳过、separator 拼接
