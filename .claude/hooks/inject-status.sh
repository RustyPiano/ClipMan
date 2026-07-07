#!/bin/bash
# SessionStart hook: inject the living project-status doc into context so a
# fresh session (or agent) starts with current background, without the user
# re-explaining. Silent no-op if the doc is missing.
cd "$CLAUDE_PROJECT_DIR" 2>/dev/null || exit 0
if [ -f docs/dev/STATUS.md ]; then
  echo "=== 项目状态（自动注入自 docs/dev/STATUS.md；维护规则见 AGENTS.md 文档维护协议）==="
  cat docs/dev/STATUS.md
fi
exit 0
