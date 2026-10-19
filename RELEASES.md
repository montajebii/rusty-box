# Releases

Release notes for The Idea Bucket.

## Release format

Every release records:

- Version (`versionName`) and `versionCode`
- What changed
- The commit(s) behind it
- Any build or release-step notes worth keeping

The version is kept consistent across `package.json` and the release build. When a release is cut, `package.json` carries `"version": "<same>"` and the release APK is built with `-PversionName=<same>`. The `versionCode` must increase on every release — Android will not upgrade an installed app otherwise.

## v1.2.0 — 2026-10-18

Capture, inbox, and source-aware ideas.

### What changed

- New ideas can now be captured with lower friction: the composer entry point has a dedicated capture action, and a page selection can be pulled in as the next idea's source line.
- New ideas can be dropped without a bucket. If no bucket is selected, the idea is filed into the inbox and the app says so plainly.
- Every idea can carry an optional source line. Right now that is the page selection at the moment of capture; later this can grow into copied text, URLs, or other origins.
- Bucket assignment is optional at capture time: ideas are valid without any bucket, and the UI says "no bucket yet" instead of treating that as missing data.
- Cards and the idea page now show the source line when an idea has one, and the empty-bucket state is more explicit.

### Why these three first

- The first job of the app is to catch a thought before it disappears. That means reducing friction at capture and not forcing a bucket decision before the idea exists.
- The second job is to make captured ideas usable later. A source line helps an idea survive context loss.
- The third is to keep the inbox honest: "no bucket" is a real, normal state, not an edge case.

### Scope of this release

From the brainstorm, these are the top three:

1. Instant capture / "Catch it now" entry point.
2. Automatic inbox for ideas with no bucket.
3. Optional source line on an idea.

Review actions, gentle review queue, random note, search/views, pins, backup, and sync were deliberately left for later.

### Commits

- `08d39e8` — docs: add README, RELEASES.md, and BUILD-NOTES.md for v1.1.0
- `dafb3ea` — visual: dark theme polish for web + Android (v1.1.0)
- `8a3b9f1` — build: add the v1.2.0 top-3 release commit

### Version

- `versionName`: 1.2.0
- `versionCode`: set at release build time with `-PversionCode=<n>`; must be higher than the previous release's code.

### Build notes

- Web build: `npm run build`
- Sync into Android: `npm run cap:sync`
- Release APK: `cd android && ./gradlew assembleRelease -PversionName=1.2.0 -PversionCode=<n>`
- Pre-release gate: `bash check-apk.sh <apk>`

## v1.1.0 — 2026-10-18

Dark theme polish for the web app and the Android app.

### What changed

- Darker, cleaner surfaces for the web app's main stage, composer, and drawer, with small contrast and spacing refinements.
- Matching dark theme pass for the Android app: splash screen, launcher icon background, and app theme now sit on the same palette as the web app.
- No bucket or feature changes in this release.

### Commits

- `08d39e8` — docs: add README, RELEASES.md, and BUILD-NOTES.md for v1.1.0
- `dafb3ea` — visual: dark theme polish for web + Android (v1.1.0)

### Version

- `versionName`: 1.1.0
- `versionCode`: set at release build time with `-PversionCode=<n>`; must be higher than the previous release's code.

### Build notes

- Web build: `npm run build`
- Sync into Android: `npm run cap:sync`
- Release APK: `cd android && ./gradlew assembleRelease -PversionName=1.1.0 -PversionCode=<n>`
- Pre-release gate: `bash check-apk.sh <apk>`

## v1.0.0 — 2026-10-18

First public release.

### What changed

- Initial release of The Idea Bucket: a minimal, local-first, dark-themed idea capture app for the web and Android.
- Core features: capture, buckets, dip for inspiration, search in the drawer, edit, copy, share, and export to JSON or Markdown.

### Commits

- `08d39e8` — docs: add README, RELEASES.md, and BUILD-NOTES.md for v1.1.0
- `dafb3ea` — visual: dark theme polish for web + Android (v1.1.0)

### Version

- `versionName`: 1.0.0
- `versionCode`: set at release build time with `-PversionCode=<n>`; must be higher than the previous release's code.

### Build notes

- Web build: `npm run build`
- Sync into Android: `npm run cap:sync`
- Release APK: `cd android && ./gradlew assembleRelease -PversionName=1.0.0 -PversionCode=<n>`
- Pre-release gate: `bash check-apk.sh <apk>`

---

## release-v1.2.0.sh

This is the release script for v1.2.0. It belongs with this release's documentation, not to the changelog sections above.

```bash
#!/usr/bin/env bash
set -euo pipefail

VERSION_NAME="1.2.0"
# versionCode must be a positive integer and must increase on every release.
# Pick it for this release before running this script.
VERSION_CODE="${VERSION_CODE:-}"

if [[ -z "$VERSION_CODE" || ! "$VERSION_CODE" =~ ^[1-9][0-9]*$ ]]; then
  echo "Set VERSION_CODE. For this release use a number higher than the previous one." >&2
  exit 1
fi

echo "==> Bumping package.json to ${VERSION_NAME}"
node -e "
  const p = require('./package.json');
  p.version = '${VERSION_NAME}';
  require('fs').writeFileSync('package.json', JSON.stringify(p, null, 2) + '\n');
"

echo "==> Building web assets"
npm run build

echo "==> Syncing into Android"
npx cap sync android

echo "==> Pre-release checklist"
echo "   - package.json version: ${VERSION_NAME}"
echo "   - RELEASES.md: confirm the v1.2.0 section matches"
echo "   - README.md / BUILD-NOTES.md: confirm they still say ${VERSION_NAME}"
echo "   - APK release command:"
echo "       cd android && ./gradlew assembleRelease -PversionName=${VERSION_NAME} -PversionCode=${VERSION_CODE}"
echo "   - Then gate it: bash check-apk.sh android/app/build/outputs/apk/release/app-release.apk"
```
