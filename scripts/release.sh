#!/usr/bin/env bash
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

sedi() { if sed --version >/dev/null 2>&1; then sed -i "$@"; else sed -i '' "$@"; fi; }

CUR=$(grep -m1 '"version"' package.json | sed -E 's/.*"version": *"([^"]+)".*/\1/')
require_version_not_older "$VERSION" "$CUR"
echo "Bumping $CUR -> $VERSION"

sedi -E "s/^(  \"version\": \")[^\"]+\"/\1${VERSION}\"/" package.json
sedi -E "s/^(  \"version\": \")[^\"]+\"/\1${VERSION}\"/" src-tauri/tauri.conf.json
sedi -E "s/^(version = \")[^\"]+\"/\1${VERSION}\"/" src-tauri/Cargo.toml
awk -v v="$VERSION" '
  prev == "name = \"clipman\"" && /^version = / { sub(/"[^"]+"/, "\"" v "\"") }
  { print; prev = $0 }
' src-tauri/Cargo.lock > src-tauri/Cargo.lock.tmp && mv src-tauri/Cargo.lock.tmp src-tauri/Cargo.lock

for f in README.md README_EN.md; do
  [ -f "$f" ] || continue
  sedi -E "s/ClipMan_[0-9]+\.[0-9]+\.[0-9]+_/ClipMan_${VERSION}_/g" "$f"
done

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

echo ""
bash "$ROOT/scripts/check-versions.sh" "$VERSION"
echo ""
echo "Done. Next:"
echo "  1. edit $NOTES"
echo "  2. git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock README.md README_EN.md $NOTES"
echo "     git commit -m \"release: v${VERSION}\""
echo "  3. git tag v${VERSION}"
echo "     git push --atomic origin HEAD:main refs/tags/v${VERSION}"
