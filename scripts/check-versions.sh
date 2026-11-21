#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source "$ROOT/scripts/version-utils.sh"

pkg=$(grep -m1 '"version"' package.json | sed -E 's/.*"version": *"([^"]+)".*/\1/')
conf=$(grep -m1 '"version"' src-tauri/tauri.conf.json | sed -E 's/.*"version": *"([^"]+)".*/\1/')
cargo=$(grep -m1 '^version = ' src-tauri/Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')
lock=$(grep -A1 -m1 '^name = "clipman"$' src-tauri/Cargo.lock | grep '^version' | sed -E 's/.*"([^"]+)".*/\1/')

printf '  package.json    : %s\n' "$pkg"
printf '  tauri.conf.json : %s\n' "$conf"
printf '  Cargo.toml      : %s\n' "$cargo"
printf '  Cargo.lock      : %s\n' "$lock"

fail=0
for v in "$pkg" "$conf" "$cargo" "$lock"; do
  validate_version "$v"
  [ "$v" = "$pkg" ] || fail=1
done
if [ "$fail" -ne 0 ]; then
  echo "::error::version mismatch across manifest files — run scripts/release.sh to sync them" >&2
  exit 1
fi

if [ "$#" -ge 1 ]; then
  expected="${1#v}"
  validate_version "$expected"
  if [ "$pkg" != "$expected" ]; then
    echo "::error::manifest version ($pkg) does not match expected/tag ($expected)" >&2
    exit 1
  fi
fi

echo "OK: all manifests at $pkg"
