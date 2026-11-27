# 0001 代码不写注释，设计决定写进 ADR

- 状态：已采纳（2026-11-22）

## 背景

2026-11-22 逐条审查了全部代码注释。ADR 指 Architecture Decision Record（架构决定记录）。大多数注释只是重复代码；少数记录了从代码看不出的设计决定；还有几处在为权宜实现开脱，让同样的写法在后续修改里被照搬。注释和代码一起修改时容易漏改，久而久之就会过时。

## 决定

- 代码、测试、脚本和配置文件里不写注释。只重复代码或已经过时的注释直接删除；为权宜实现开脱的注释删除后提 GitHub Issue；设计决定写进 `docs/adr/`。
- `src/lib/bindings/` 由 ts-rs 生成，文件头的生成说明不算注释，ESLint 和 Prettier 都跳过这个目录。
- 工具指令不算注释：`eslint-disable`、`@ts-expect-error`、`svelte-ignore`、`prettier-ignore`、`/// <reference>`，以及脚本首行的 shebang。
- `bun run lint` 用 `eslint.config.js` 里的 `local/no-comments` 规则检查全部 JS/TS/Svelte 文件，覆盖 `<script>`、模板和 `<style>`。`cargo test` 运行 `src-tauri/tests/no_comments.rs`，用 rustc 的 lexer（`ra-ap-rustc_lexer`）检查 Rust 文件。
- 两个检查都设上限，注释数量只能减少：ESLint 用 `eslint-suppressions.json`，Rust 用 `src-tauri/tests/comment-suppressions.json`。某个文件的注释超过上限时检查失败；注释减少后要收紧上限（`bunx eslint . --prune-suppressions`，`PRUNE_COMMENT_SUPPRESSIONS=1 cargo test --test no_comments`），否则检查同样失败。
- 普通 CSS 文件和 Python、Shell、Swift、YAML、TOML、tsconfig 没有自动检查，按同一规则人工把关，`/delete-comment` 审查时一并处理。

## 影响

理解代码依靠命名、结构、测试和 ADR。ADR 用函数名或文件名引用代码，不写行号。上限文件不能手工调高，也不运行 `--suppress-all`。
