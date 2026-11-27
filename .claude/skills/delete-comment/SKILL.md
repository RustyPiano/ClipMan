---
name: delete-comment
description: 审查 ClipLeaf 的代码注释并删除：同义重复和过时的直接删，为权宜实现开脱的删后提 GitHub Issue，设计决定迁入 docs/adr/，最后收紧注释检查的上限。用户输入 /delete-comment 或要求清理注释时使用。
---

# 删除注释

规则见 `docs/adr/0001-no-code-comments.md`。中间文件放在已忽略的 `.comment-audit/`。

## 1. 列出注释

- JS/TS/Svelte：`mkdir -p .comment-audit && echo '{}' > .comment-audit/empty.json && bunx eslint . --suppressions-location .comment-audit/empty.json`，报告里的 `local/no-comments` 即全部注释。
- Rust：先 `cp src-tauri/tests/comment-suppressions.json .comment-audit/`，把原文件改成 `{}`，运行 `cargo test --manifest-path src-tauri/Cargo.toml --test no_comments`，失败信息列出每条注释的 `文件:行`；看完把备份复制回去。
- 没有自动检查的文件：`rg -n -g '*.{css,py,sh,swift,yml,yaml,toml}' -g 'tsconfig*.json' -e '^\s*(#|//|/\*)' -e '\s(#|//)\s' .`。逐条确认，heredoc、字符串和 shebang 不算注释；`run.py` 的 docstring 被用作用法提示，属于代码。

## 2. 分类

每条注释先读周围代码，再归入一类：

- 同义重复：重复代码已经表达的内容。
- 过时：描述的代码或实现已经不存在或不再成立。
- 开脱：为权宜实现解释或辩护（“暂时”“固定等待”“保持一致”“可以接受”“ponytail:” 等）。
- 设计决定：记录不明显且重要的取舍或约束。

## 3. 处理

- 同义重复、过时：删除。
- 开脱：删除，并提 GitHub Issue。先用 `gh issue list --state all --search <关键词>` 查重，已有就引用。Issue 写清文件和函数、现状、风险和验收方式，不写行号。
- 设计决定：并入 `docs/adr/` 同主题的 ADR，或新建 `docs/adr/NNNN-<主题>.md`（状态、背景、决定、影响）。按函数名引用代码，不写行号。
- 删除会让代码出错时（例如空 catch 块依赖注释通过 `no-empty`），先保留注释，在 Issue 里写明，修复代码时一起删除。
- `AGENTS.md`、`docs/dev/STATUS.md` 里指向被删注释的说明改为指向 ADR。

## 4. 收紧上限并验证

- `bun run format && bunx prettier --write tests e2e`，`cargo fmt --manifest-path src-tauri/Cargo.toml`。
- `bunx eslint . --prune-suppressions`，`PRUNE_COMMENT_SUPPRESSIONS=1 cargo test --manifest-path src-tauri/Cargo.toml --test no_comments`。上限只能下降：不运行 `--suppress-all`，不手工调高 `comment-suppressions.json`。
- 运行 `AGENTS.md` 列出的全部质量检查，更新 `docs/dev/STATUS.md`。

## 5. 汇报

汇报内容：各类注释数量、删除行数、新提或引用的 Issue、改动过的 ADR 和剩余上限。
