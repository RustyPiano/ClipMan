---
name: delete-comment
description: Audit ClipMan code comments, delete redundant or stale prose, file issues for justified shortcuts, migrate decisions into ADRs, and shrink the comment lint baseline. Use for /delete-comment or a requested comment cleanup.
---

Run `bun scripts/lint-comments.js --list` at the repository root. It reports file, line, zero-based UTF-16 column, and text for each comment fragment. Group consecutive fragments belonging to the same comment. Check the file and callers before classifying it.

- **Tautology** — remove a restatement of visible behavior.
- **Stale** — verify the discrepancy against the implementation and remove the obsolete claim.
- **Excuse** — identify the concrete shortcut and its consequence. For a cleanup that authorizes filing issues, reuse an existing matching issue or file one with the affected symbol and a verifiable acceptance condition. Remove the comment and record the issue number.
- **Decision Record** — move the constraint, chosen behavior, and consequence into the relevant decision in `docs/adr/`. Preserve platform-specific and unsafe-code assumptions.

Preserve executable directives, generated headers, literal data and inline code. A review-only request produces findings without edits or external issue creation. When deleting grandfathered comments, remove their matching entries from `scripts/comment-baseline.json`. Never regenerate a larger baseline to make lint pass. For a repository-wide pass check the file types from `git ls-files` against the scanner’s language map.

Run `bun run lint` and the applicable project checks. Report the reviewed revision, classifications, ADRs, issue links, and remaining comments. Count standalone physical lines for the savings estimate. Removing inline prose leaves the code line in place.
