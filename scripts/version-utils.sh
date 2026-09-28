#!/usr/bin/env bash

validate_version() {
  if ! [[ "$1" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
    echo "::error::版本必须为 X.Y.Z，不带 v 或前导零：$1" >&2
    return 1
  fi
}

require_version_not_older() {
  local candidate="$1" current="$2" index
  local candidate_parts current_parts
  validate_version "$candidate" || return 1
  validate_version "$current" || return 1
  IFS=. read -r -a candidate_parts <<< "$candidate"
  IFS=. read -r -a current_parts <<< "$current"
  for index in 0 1 2; do
    if ((candidate_parts[index] > current_parts[index])); then
      return 0
    fi
    if ((candidate_parts[index] < current_parts[index])); then
      echo "::error::版本不能从 $current 倒退到 $candidate" >&2
      return 1
    fi
  done
}

require_unused_release_tag() {
  local status
  if git show-ref --verify --quiet "refs/tags/v$1"; then
    echo "::error::标签 v$1 已存在；请重试该标签对应的 Release 工作流，不要重新准备版本。" >&2
    return 1
  else
    status=$?
    if [ "$status" -ne 1 ]; then
      return "$status"
    fi
  fi
}
