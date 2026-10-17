#!/usr/bin/env bash
# make-release.sh — build a versioned release APK after the full quality gate passes.
#
# Pipeline:
#   1. Stamp version (from package.json, or $1) into android/app/build.gradle
#      — versionName = the semver, versionCode auto-increments when the name changes
#   2. Build the APK          (delegates to build-apk.sh — reboot-proof, no env setup)
#   3. Run the 7-gate check   (delegates to check-apk.sh — must pass or we stop)
#   4. File it: releases/ByTheWay-v<version>-debug.apk + sha256 + RELEASES.md entry
#
# Usage:
#   bash make-release.sh              # release the version from package.json
#   bash make-release.sh 1.1.0        # release an explicit version
#   bash make-release.sh 1.1.0 --tag  # also create a local git tag v<version>
set -euo pipefail
cd "$(dirname "$0")"

VERSION="${1:-}"
TAG=false
[ "${1:-}" = "--tag" ] && { TAG=true; VERSION="${2:-}"; }
[ "${2:-}" = "--tag" ] && TAG=true

if [ -z "$VERSION" ]; then
  VERSION=$(python3 -c "import json;print(json.load(open('package.json'))['version'])")
fi
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "✗ not a semver: '$VERSION' (expect X.Y.Z)" >&2; exit 1; }

GRADLE_FILE=android/app/build.gradle
CUR_NAME=$(sed -n 's/.*versionName "\([^"]*\)".*/\1/p' "$GRADLE_FILE" | head -1)
CUR_CODE=$(sed -n 's/.*versionCode \([0-9]*\).*/\1/p' "$GRADLE_FILE" | head -1)

if [ "$CUR_NAME" != "$VERSION" ]; then
  NEW_CODE=$((CUR_CODE + 1))
  sed -i "s/versionCode $CUR_CODE/versionCode $NEW_CODE/; s/versionName \"$CUR_NAME\"/versionName \"$VERSION\"/" "$GRADLE_FILE"
  echo "Stamped: versionName $CUR_NAME → $VERSION, versionCode $CUR_CODE → $NEW_CODE"
  FINAL_CODE=$NEW_CODE
else
  echo "Version already $VERSION (versionCode $CUR_CODE) — no restamp needed"
  FINAL_CODE=$CUR_CODE
fi

echo "==> build-apk.sh"
bash build-apk.sh

echo "==> check-apk.sh (release gate — must pass)"
bash check-apk.sh by-the-way-debug.apk

mkdir -p releases
OUT="releases/ByTheWay-v$VERSION-debug.apk"
cp by-the-way-debug.apk "$OUT"
SHA=$(sha256sum "$OUT" | cut -d' ' -f1)
SIZE=$(du -h "$OUT" | cut -f1)

CHANGELOG=releases/RELEASES.md
ENTRY="| $VERSION | $CUR_CODE$( [ "$CUR_NAME" != "$VERSION" ] && echo "→$NEW_CODE" ) | $(date +%F) | $SIZE | \`$SHA\` |"
if ! grep -qF "| $VERSION |" "$CHANGELOG" 2>/dev/null; then
  [ -f "$CHANGELOG" ] || printf '# By the way (راستی) releases\n\n| Version | versionCode | Date | Size | sha256 |\n|---|---|---|---|---|\n' > "$CHANGELOG"
  echo "$ENTRY" >> "$CHANGELOG"
fi

echo
echo "✔ RELEASE READY: $OUT"
echo "  version $VERSION (code ${FINAL_CODE}), sha256 $SHA"
echo "  install:  adb install -r \"$OUT\""
if $TAG; then
  git tag -f "v$VERSION"
  echo "  tagged:   v$VERSION (local only — push with: git push origin v$VERSION)"
fi
