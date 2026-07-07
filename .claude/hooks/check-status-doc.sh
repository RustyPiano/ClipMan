#!/bin/bash
# Stop hook: block once with a reminder when uncommitted source changes are
# newer than docs/dev/STATUS.md, so the status doc never silently rots.
input=$(cat)

# stop_hook_active=true means we already blocked this stop cycle and Claude is
# finishing its follow-up — let it through to avoid an infinite loop.
if echo "$input" | jq -e '.stop_hook_active == true' >/dev/null 2>&1; then
  exit 0
fi

cd "$CLAUDE_PROJECT_DIR" 2>/dev/null || exit 0

status_file="docs/dev/STATUS.md"
if [ ! -f "$status_file" ]; then
  echo '{"decision":"block","reason":"docs/dev/STATUS.md 不存在——项目状态活文档被删除或移动了。请按 AGENTS.md 的文档维护协议恢复或重建它。"}'
  exit 0
fi

# Only look at code paths git considers modified/untracked, so build artifacts
# and target/ noise never trigger the reminder.
stale=""
while IFS= read -r f; do
  if [ -n "$f" ] && [ -f "$f" ] && [ "$f" -nt "$status_file" ]; then
    stale="$f"
    break
  fi
done < <(git status --porcelain=v1 -- src src-tauri/src src-tauri/Cargo.toml src-tauri/tauri.conf.json 2>/dev/null | cut -c4-)

if [ -n "$stale" ]; then
  reason="源码存在比 docs/dev/STATUS.md 更新的未提交改动（如 ${stale}）。请按 AGENTS.md 的文档维护协议更新 STATUS.md：刷新「当前状态/工作区/待办/已知问题」并更新日期；若确认本次改动无需记录，touch docs/dev/STATUS.md 后正常结束。"
  jq -n --arg r "$reason" '{"decision":"block","reason":$r}'
fi
exit 0
