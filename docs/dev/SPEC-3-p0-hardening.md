# SPEC-3：P0 信任底座（CI / 启动恢复 / 锁一致性 / 空间回收 / 大小上限）

> 各节独立派工，文件域互不重叠。前置阅读 docs/dev/PLAN.md。

## §1 CI 测试关卡（W1-A · Sonnet · 任务#2）

文件域：`.github/workflows/ci.yml`（新建）、`package.json`（只加 test 脚本）

### 要求
1. `package.json` scripts 加 `"test": "bun test tests/"`。
2. 新建 `.github/workflows/ci.yml`：
   - 触发：`push`（main 分支）与 `pull_request`
   - **frontend job**（ubuntu-latest）：`oven-sh/setup-bun@v2` → `bun install --frozen-lockfile` → `bun run lint` → `bun run check` → `bun test tests/` → `bun run build`
   - **rust job**（macos-14）：checkout → setup-bun → `bun install --frozen-lockfile` → `bun run build`（cargo test 需要 dist/ 存在）→ 安装 rust-toolchain.toml 固定的工具链（`dtolnay/rust-toolchain@stable` 会读 rust-toolchain.toml，或 rustup 自动识别）→ `Swatinem/rust-cache@v2`（workspaces: src-tauri）→ `cargo fmt --check` → `cargo clippy --all-targets -- -D warnings` → `cargo test`（均在 src-tauri/ 下）
   - `concurrency` 取消同分支旧跑
3. 本地验证（不要真跑 cargo build/clippy——协调者统一跑）：
   - `bun run lint && bun run check && bun test tests/` 全通过
   - `cd src-tauri && cargo fmt --check` 通过（fmt 不编译，可跑）；若 fmt 有 diff，**如实报告，不要自行大面积重排**（可能与并行 agent 冲突）
4. 若怀疑 clippy `-D warnings` 会挂（现有代码有 warning），把 clippy 步骤先写成不带 `-D warnings` 并在 Deviations 里说明，协调者验证后再收紧。

### 验收
- [ ] yaml 语法有效（可用 `bun x yaml-lint` 或等价方式自查）
- [ ] 本地前端四连（lint/check/test/build）全绿
- [ ] 不改 package.json 其他内容、不动 release.yml

## §2 启动崩溃恢复（W2-B · Sonnet · 任务#3）

文件域：`src-tauri/src/main.rs`、`src-tauri/src/migration.rs`、`src-tauri/src/storage.rs`（仅 `ClipStorage::new` 签名）

### 现状
`main.rs:103-122`：`app_data_dir().expect` / `create_dir_all().expect` / `db_path.to_str().unwrap()` / `ClipStorage::new().expect`；release 是 `panic="abort"` → 任何一步失败应用直接死亡且无恢复入口。`migration::get_data_directory`（migration.rs:278-284）对 custom_data_path 盲目返回。

### 要求
1. `ClipStorage::new` 签名改为 `new(db_path: &Path)`，内部 `Connection::open(db_path)`（消除 to_str().unwrap() 的非 UTF-8 隐患）；同文件 `data_dir_for_db_path` 相应调整；测试调用点同步更新。
2. main.rs setup 中的存储初始化提取为函数，实现降级链：
   - custom 目录 create_dir_all 失败或 ClipStorage::new 失败 → `log::warn` + 弹一次阻塞对话框（tauri_plugin_dialog，文案说明"自定义数据目录不可用，本次使用默认目录"）→ 回落默认目录重试
   - 默认目录 ClipStorage::new 失败（疑似损坏）→ 把 `clipman.db` 及 `-wal/-shm/-journal` 改名为 `clipman.db.corrupt-<unix秒>`（复用 storage.rs 现有 sidecar 工具函数，如需 pub 化在 Deviations 说明）→ 重建全新库 → 对话框告知"历史库损坏已重置，旧文件保留为 …corrupt-…"
   - 默认目录 create_dir_all 也失败 → 对话框展示错误后 `std::process::exit(1)`（唯一允许的退出路径，不再 panic）
3. 降级链单元测试：损坏 db 文件（写垃圾字节）→ 初始化函数返回全新可用存储且 corrupt 备份文件存在。对话框逻辑用 `#[cfg(test)]` 旁路或依赖注入绕开。

### 验收
- [ ] main.rs setup 中不再有 expect/unwrap（除 tauri 框架强制的 `run().expect`）
- [ ] cargo test 全绿含新增测试；clippy 无新增 warning
- [ ] 三种失败注入（custom 目录不存在且不可创建 / db 损坏 / 非 UTF-8 路径）都能启动成功或优雅退出

## §3 settings.rs safe_lock（W1-C · Sonnet · 任务#14）

文件域：`src-tauri/src/settings.rs`

- `settings.rs:182, 189, 254, 257` 的 `.lock().unwrap()` 全部替换为 `crate::safe_lock(...)`（模式见 main.rs:33-38）。
- 检查同文件是否还有其他裸 lock（grep `.lock()`），一并统一。
- 不改任何行为、不改公共 API。验收：`cargo test` 全绿、`grep -n "lock().unwrap()" settings.rs` 为空。

## §4 数据库空间回收（W2-C 前半 · Sonnet · 任务#9）

文件域：`src-tauri/src/storage.rs`

- `ClipStorage::new`：打开后检查 `PRAGMA auto_vacuum`；若不是 2（INCREMENTAL）→ `PRAGMA auto_vacuum=INCREMENTAL` + 一次性 `VACUUM`（一次性迁移，日志记录耗时）。
- `prune_history_with_conn`、`clear_all_with_conn`、`clear_non_pinned_with_conn`、`delete_with_conn` 之后（事务提交后）执行 `PRAGMA incremental_vacuum`。注意 incremental_vacuum 不能在事务内跑——放在调用方 commit 之后，封装 `fn reclaim_space(&self)` 容错（失败仅 log）。
- 测试：插入大量带图数据 → 删除 → db 文件尺寸显著回落（对比 page_count 更稳定：`PRAGMA page_count * page_size` 断言下降）。

## §5 单条大小上限 + 图片降采样（W2-C 后半 · Sonnet · 任务#7）

文件域：`src-tauri/src/{clipboard,settings}.rs`

- settings 增加（含默认值与 clamp，模式照抄 max_history_items）：
  - `max_text_bytes: usize`（默认 2_000_000，clamp 4096..=50_000_000）——超限的 Text/Files **跳过采集**并 log
  - `max_image_dimension: u32`（默认 4096，clamp 512..=16384，0 表示不缩）——超限图片按最长边等比降采样后再存（Lanczos3，复用现有 image 依赖）
  - html 附带内容超过 `max_text_bytes` → 丢弃 html 只存纯文本（log debug）
- 前端设置 UI 本节不做（Wave 3 统一）。
- 测试：超限文本被跳过；大图降采样后尺寸 ≤ 上限且缩略图仍 256。

### §4/§5 共同验收
- [ ] cargo test 全绿、clippy 无新增 warning
- [ ] 与 W1-B 合入后的 clipboard.rs 无冲突（W2 启动时以最新代码为准）
