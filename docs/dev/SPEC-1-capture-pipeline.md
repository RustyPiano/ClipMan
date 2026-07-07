# SPEC-1：采集管线重构 + Files 类型 + HTML 富文本（后端）

> Wave 1-B · 模型：Opus · 文件域：`src-tauri/src/{storage,clipboard,paste,commands,tray}.rs`
> 对应任务：#4 单一代表格式、#5 文件类型、#6 富文本（后端部分）
> 前置阅读：docs/dev/PLAN.md 的架构决策 D1-D6 与 Agent 工作守则

## 0. 现状与问题

- `clipboard.rs:49-96`（Handler::on_clipboard_change）与 `clipboard.rs:232-270`（polling 回退）对同一次剪贴板变化**独立地**读 text 和 image，各自入库 → 一次复制两条记录；Finder 复制文件 → 文件名文本 + 文件图标图片两条垃圾记录。
- `ContentType` 只有 Text/Image（storage.rs:11-14），无文件、无富文本。
- arboard 已锁定 3.6.1，`Get`/`Set` 均支持 `text/html/image/file_list`（已核实 docs.rs）。

## 1. storage.rs 改动

### 1.1 ContentType
```rust
pub enum ContentType { Text, Image, Files }
// as_db_value: Files => "files"；from_db_value: "files" => Files（未知值仍回落 Text）
```

### 1.2 ClipItem / ClipPreviewItem / FrontendClipItem
- `ClipItem` 新增字段 `pub html: Option<String>`（放在 `source_app` 之后，保持 serde camelCase）。
- `ClipPreviewItem` 新增 `pub has_html: bool`。
- `FrontendClipItem` 新增 `pub has_html: bool`（序列化为 `hasHtml`）。
  - `From<ClipItem>`：`has_html: item.html.is_some()`；content 映射中 Files 走与 Text 相同的 base64 分支。
  - `from_preview`：透传 `item.has_html`；Files 与 Text 同分支（base64(preview_content)）。
  - `from_full_text`（storage.rs:194）：放宽为 Text **和 Files** 都返回完整内容（QuickBar 预览面板要显示完整路径列表），Image 仍返回 None。
- `ClipPreviewItem::from_clip_item_with_id`（storage.rs:139）：preview_content 对 Text|Files 都取前 `TEXT_PREVIEW_BYTES`；`has_html: item.html.is_some()`。

### 1.3 列常量与行映射（注意列序，新增列一律排在 source_app 之后）
- `CLIP_COLUMNS` / `CLIP_COLUMNS_WITH_ALIAS`：追加 `html`（索引 10），`clip_from_row` 读 `row.get(10)?`。
- `CLIP_PREVIEW_COLUMNS` / `_WITH_ALIAS`：
  - `CASE WHEN content_type = 'text'` → `CASE WHEN content_type IN ('text','files')`（preview_content 对 files 也取前 4096 字节路径文本）
  - 追加 `(html IS NOT NULL) AS has_html`（索引 10），`preview_from_row` 读 bool（`row.get::<_, i32>(10)? != 0` 或 `row.get::<_, bool>`，与现有 is_pinned 读法一致用 i32）。

### 1.4 schema
- `initialize_schema` 的 `add_column_if_missing` 列表追加 `("html", "TEXT")`。CREATE TABLE 语句同步加 `html TEXT` 列（新库路径）。
- **不改 user_version**（决策 D4）。

### 1.5 insert 与去重（storage.rs:267-319）
- INSERT 语句加入 `html`（?12）。
- 重复命中分支：不再只调 `update_timestamp_with_conn`，改为：
```sql
UPDATE clips SET timestamp = ?1, html = COALESCE(?2, html) WHERE id = ?3
```
（新私有 fn `refresh_duplicate_with_conn(conn, id, timestamp, html: Option<&str>)`；`update_timestamp_with_conn` 保留给 paste 的 touch 用）

### 1.6 FTS 与搜索
- `search_text_for_fts`：`Files => String::from_utf8_lossy(content).into_owned()`（路径可搜）。
- `fts_payload_for_rowid_with_conn` / `sync_fts_for_clip_id_with_conn` 中的 `CASE WHEN content_type = 'text'` → `IN ('text','files')`。
- `search_with_like` / `search_previews_with_like` 中 `content_type = 'text'` → `content_type IN ('text','files')`。

### 1.7 路径编解码助手（pub，含单元测试）
```rust
pub fn join_file_paths(paths: &[String]) -> String   // paths.join("\n")
pub fn split_file_paths(content: &str) -> Vec<String> // 按 '\n' 拆、过滤空行
```

## 2. clipboard.rs 改动（核心）

### 2.1 快照读取（单一代表格式，决策 D1）
```rust
enum ClipboardSnapshot {
    Files(Vec<String>),                              // 绝对路径，to_string_lossy
    Text { text: String, html: Option<String> },
    Image(arboard::ImageData<'static>),
}

fn read_clipboard_snapshot(clipboard: &mut Clipboard) -> Option<ClipboardSnapshot> {
    // 1) file_list 非空 → Files（不再看 text/image：Finder 的文件名文本与图标是派生噪音）
    // 2) get_text 非空   → Text，并尝试 get().html()（Err 或空串 → None）
    // 3) get_image 成功  → Image
    // 全失败 → None
}

fn snapshot_marker(snapshot: &ClipboardSnapshot) -> CopyMarker {
    // Files → CopyMarker::from_payload(Files, join_file_paths(paths).as_bytes())
    // Text  → from_payload(Text, text.as_bytes())        ← 无视 html（决策 D5）
    // Image → 现有 image_marker()（规范化 RGBA）
}
```
注意 image 读取必须保持**惰性**（只有前两级都未命中才调 get_image），大图读取昂贵。

### 2.2 Handler 与 polling 统一
- `Handler` 字段 `last_text: String` + `last_image_marker: Option<CopyMarker>` **合并替换**为 `last_marker: Option<CopyMarker>`；polling 循环同样。
- 事件/轮询共用一个处理函数：
```rust
fn handle_clipboard_event(
    app_handle: &AppHandle,
    last_copied_by_us: &Arc<Mutex<Option<CopyMarker>>>,
    running: &Arc<AtomicBool>,
    clipboard: &mut Clipboard,
    last_marker: &mut Option<CopyMarker>,
)
```
流程：should_ignore（保持现有 concealed 检查在最前）→ read_clipboard_snapshot → 计算 marker → 与 `last_marker` 相同则跳过 → 更新 `last_marker` → is_self_copied 检查 → 按类型分发。

### 2.3 分发函数
- `process_text_change` 增加 `html: Option<String>` 参数，写入 `ClipItem.html`。
- 新增 `process_files_change(app, paths: Vec<String>, ...)`：content = `join_file_paths` 的字节，`content_type: Files`，`thumbnail: None`，`html: None`，`source_app: frontmost_app_name()`。
- `process_image_change` 保持现有异步处理，ClipItem 补 `html: None`。
- 自复制跳过逻辑（is_self_copied）保持逐 marker 判断，在分发前做（见 2.2 流程）——注意现有代码是在各 process 函数内部判断，统一挪到分发前也可以，二选一但要保持"跳过时不打断 last_marker 更新"（否则粘贴后第一次真实复制会被误吞）。

## 3. paste.rs 改动

### 3.1 写回（write_clip_to_system_clipboard）
- 签名追加 `plain_text_only: bool`（`paste_clip` 链路透传；`copy_clip_to_clipboard_internal` 传 false）。
- match 增加 Files 分支 `write_files`：
  - `split_file_paths` → `Vec<PathBuf>` → `clipboard.set().file_list(...)`（按 arboard 3.6 实际签名调整）
  - marker = `from_payload(Files, join_file_paths(...).as_bytes())`，写入前 set、失败回滚（模式照抄 write_text）
  - `set().file_list` 返回 Err 时：log warn 并降级 `set_text(joined)`（marker 换成 Text 的）
- `write_text`：当 `item.html` 为 Some 且 `!plain_text_only` 时，用 `clipboard.set().html(html, Some(text))`；否则 `set_text`。**marker 恒为 `from_payload(Text, text.as_bytes())`**（决策 D5：监控端 get_text 读到的是 alt 纯文本）。

### 3.2 paste_clip 命令链
- `paste.rs::paste_clip` 签名追加 `plain: bool`；`commands.rs::paste_clip` 命令追加 `plain: Option<bool>` 参数（`.unwrap_or(false)`），保持对旧前端调用兼容。

## 4. commands.rs / tray.rs 收尾

- `notify_copied`（commands.rs:319-330）：加 `ContentType::Files => "文件已复制到剪贴板"`（现有硬编码中文风格保持一致，i18n 债务后续统一还）。
- `tray.rs::truncate_content`（tray.rs:121-159）：match 改为 `ContentType::Text | ContentType::Files` 共用文本分支（换行已被替换为空格，路径列表可读）；Image 分支不变。

## 5. 测试要求

更新：所有 `ClipItem` 字面量补 `html: None`（storage tests 的 `test_item`、clipboard tests 等）。

新增（最少集合）：
1. storage：Files 项 insert → `get_by_id` 往返 content_type/content 不变；`search("路径关键词")` 能命中 Files 项。
2. storage：html 列往返（insert 带 html → get_by_id 读回）；重复插入时 `COALESCE` 语义（带 html 的重复刷新 html，不带 html 的重复保留旧 html）。
3. storage：`join_file_paths`/`split_file_paths` 往返 + 空行过滤。
4. storage：preview 对 Files 返回路径文本、`has_html` 位正确。
5. clipboard：`snapshot_marker` 对三种变体的哈希与 `CopyMarker::from_payload`/`from_normalized_image_parts` 一致；Text 的 marker 与 html 无关（同 text 不同 html → 同 marker）。
6. tray：`truncate_content` Files 分支输出路径文本。

## 6. 验收标准（协调者逐条核验）

- [ ] `cargo test` 全绿（含新增测试），`cargo clippy --all-targets` 无新增 warning
- [ ] 一次剪贴板变化最多产生一条记录（代码路径上不存在 text 与 image 双入库的可能）
- [ ] file_list 命中时不读 text/image；text 命中时不读 image（惰性）
- [ ] 粘贴带 html 的 Text 项：剪贴板上同时有 html 与纯文本 alt；自复制标记按纯文本计算
- [ ] 粘贴 Files 项：剪贴板上是文件列表（可在 Finder ⌘V 出真实文件）
- [ ] 旧数据库（无 html 列）打开自动迁移，现有数据不丢
- [ ] `paste_clip` 命令不带 `plain` 参数时行为与现在完全一致（前端未升级也不坏）
- [ ] Deviations 一节如实记录所有偏差

## 7. 明确不做（越界即打回）

- 不改前端任何文件（Wave 2）
- 不做大小上限/降采样（#7）、不动 max_history_items 逻辑
- 不做 RTF 读取、不做采集顺序设置项
- 不改 main.rs / window.rs / settings.rs / migration.rs
