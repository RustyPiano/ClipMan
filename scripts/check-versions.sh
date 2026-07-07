#!/usr/bin/env bash
# Assert the app version is identical across every manifest that carries it:
# package.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml and the
# `clipman` entry in src-tauri/Cargo.lock.
#
# Usage:
#   scripts/check-versions.sh            # just assert the four agree
#   scripts/check-versions.sh 2.2.0      # also assert they equal this value
#   scripts/check-versions.sh v2.2.0     # leading 'v' is stripped (release tag)
#
# Portable: uses only grep/sed (no jq, no awk arrays) so it runs the same on a
# BSD (macOS) shell and on GitHub's Ubuntu runners.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

pkg=$(grep -m1 '"version"' package.json | sed -E 's/.*"version": *"([^"]+)".*/\1/')
conf=$(grep -m1 '"version"' src-tauri/tauri.conf.json | sed -E 's/.*"version": *"([^"]+)".*/\1/')
cargo=$(grep -m1 '^version = ' src-tauri/Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')
lock=$(grep -A1 -m1 '^name = "clipman"$' src-tauri/Cargo.lock | grep '^version' | sed -E 's/.*"([^"]+)".*/\1/')

printf '  package.json    : %s\n' "$pkg"
printf '  tauri.conf.json : %s\n' "$conf"
printf '  Cargo.toml      : %s\n' "$cargo"
printf '  Cargo.lock      : %s\n' "$lock"

fail=0
for v in "$conf" "$cargo" "$lock"; do
  [ "$v" = "$pkg" ] || fail=1
done
if [ "$fail" -ne 0 ]; then
  echo "::error::version mismatch across manifest files — run scripts/release.sh to sync them" >&2
  exit 1
fi

if [ "$#" -ge 1 ]; then
  expected="${1#v}"
  if [ "$pkg" != "$expected" ]; then
    echo "::error::manifest version ($pkg) does not match expected/tag ($expected)" >&2
    exit 1
  fi
fi

echo "OK: all manifests at $pkg"
