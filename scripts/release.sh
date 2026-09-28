#!/usr/bin/env bash
# Bump the app version everywhere and scaffold release notes for it.
#
# Usage:  scripts/release.sh <X.Y.Z>      (semver, no leading 'v')
#
# 只修改文件，不提交、创建标签或推送。依赖 Git 和系统文本工具，无需 Rust/Node。
# 后续操作：
#
#   1. edit release_notes_<X.Y.Z>.md
#   2. git add package.json src-tauri/{tauri.conf.json,Cargo.toml,Cargo.lock} \
#        README.md README_EN.md release_notes_<X.Y.Z>.md
#      git commit -m "release: vX.Y.Z"
#   3. git tag vX.Y.Z
#      git push --atomic origin HEAD:main refs/tags/vX.Y.Z
#      标签触发 Release，并检查该提交后构建草稿。
#
# The "Prepare Release" GitHub workflow calls this script and does steps 1-3 for
# you (see .github/RELEASE_GUIDE.md).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source "$ROOT/scripts/version-utils.sh"

VERSION="${1:-}"
if [ "$#" -ne 1 ]; then
  echo "Usage: scripts/release.sh <X.Y.Z>  (semver, no leading 'v')" >&2
  exit 1
fi
validate_version "$VERSION"
require_unused_release_tag "$VERSION"

# In-place sed that works on both GNU (Linux) and BSD (macOS) sed.
sedi() { if sed --version >/dev/null 2>&1; then sed -i "$@"; else sed -i '' "$@"; fi; }

CUR=$(grep -m1 '"version"' package.json | sed -E 's/.*"version": *"([^"]+)".*/\1/')
require_version_not_older "$VERSION" "$CUR"
echo "Bumping $CUR -> $VERSION"

# --- version manifests -------------------------------------------------------
# package.json + tauri.conf.json: the single top-level "version" key (2-space
# indent). Anchoring to `^  "version": "` avoids touching dependency versions.
sedi -E "s/^(  \"version\": \")[^\"]+\"/\1${VERSION}\"/" package.json
sedi -E "s/^(  \"version\": \")[^\"]+\"/\1${VERSION}\"/" src-tauri/tauri.conf.json
# Cargo.toml [package] version: the only `version = "..."` at column 0.
sedi -E "s/^(version = \")[^\"]+\"/\1${VERSION}\"/" src-tauri/Cargo.toml
# Cargo.lock: the version line immediately following the clipman package name.
awk -v v="$VERSION" '
  prev == "name = \"clipman\"" && /^version = / { sub(/"[^"]+"/, "\"" v "\"") }
  { print; prev = $0 }
' src-tauri/Cargo.lock > src-tauri/Cargo.lock.tmp && mv src-tauri/Cargo.lock.tmp src-tauri/Cargo.lock

# --- README download filenames (both languages) ------------------------------
# Self-healing: rewrites whatever version is in the ClipMan_X.Y.Z_ filenames, so
# it fixes stale READMEs even if they drifted from the manifest. The version
# badge is dynamic (shields github/v/release) and needs no bump.
for f in README.md README_EN.md; do
  [ -f "$f" ] || continue
  sedi -E "s/ClipMan_[0-9]+\.[0-9]+\.[0-9]+_/ClipMan_${VERSION}_/g" "$f"
done

# --- release notes scaffold --------------------------------------------------
NOTES="release_notes_${VERSION}.md"
if [ ! -f "$NOTES" ]; then
  cat > "$NOTES" <<EOF
## ClipMan v${VERSION}

### ✨ 新功能 / New

-

### 🐛 修复 / Fixes

-

### 🔧 其他 / Other

-
EOF
  echo "Scaffolded $NOTES — fill it in before tagging (release.yml requires it)."
fi

# --- verify + next steps -----------------------------------------------------
echo ""
bash "$ROOT/scripts/check-versions.sh" "$VERSION"
echo ""
echo "Done. Next:"
echo "  1. edit $NOTES"
echo "  2. git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock README.md README_EN.md $NOTES"
echo "     git commit -m \"release: v${VERSION}\""
echo "  3. git tag v${VERSION}"
echo "     git push --atomic origin HEAD:main refs/tags/v${VERSION}"
