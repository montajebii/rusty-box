# Releases

Release notes for The Idea Bucket.

## Release format

Every release records:

- Version (`versionName`) and `versionCode`
- What changed
- The commit(s) behind it
- Any build or release-step notes worth keeping

The version is kept consistent across `package.json` and the release build. When a release is cut, `package.json` carries `"version": "<same>"` and the release APK is built with `-PversionName=<same>`. The `versionCode` must increase on every release — Android will not upgrade an installed app otherwise.

## v2.0.0 — 2026-10-25

The pure-Rust rewrite. **Breaking change:** the app is no longer a single
HTML file wrapped in Capacitor; it is a Rust web server with a fully
server-rendered, zero-JavaScript UI.

### What changed

- The entire application is rewritten in Rust as a Cargo workspace:
  `ideabucket-core` (domain: model, buckets, store, search, dip, exports,
  import) and `ideabucket-server` (axum + askama). No JavaScript ships to
  the client at all — v1's composer, drawer, idea page, toasts and
  animations are replaced by forms, links, CSS animations and flash toasts.
- Data stays drop-in compatible: the store file and the JSON export keep
  v1's exact payload shape (`id/text/color/created/lastViewed/buckets/source`,
  camelCase), so a v1 browser export imports into v2 and a v2 export imports
  back into v1.
- **New: JSON import** (merge or replace). This doubles as the official
  browser → server migration path: paste a v1 `idea-bucket.json` export.
- Bucket colors, glass liquid level, blob layout and the Markdown export
  format are exact ports of v1's formulas; dates render locale-neutral as
  ISO `YYYY-MM-DD` instead of the browser locale's short date.
- Storage moves from localStorage to a JSON file with atomic saves
  (temp + rename). A corrupt file is quarantined as
  `<file>.corrupt-backup`, exactly like v1's corrupt-payload quarantine.
- Every capture/edit/delete/bucket action is validated server-side with the
  same rules v1 enforced in the UI: non-empty text, 280-char capture limit,
  2000-char edit limit, bucket names cleaned to 24 chars and deduped.
- Deleting a bucket still only unfiles its ideas — never deletes them.
- The v1 web app and Android wrapper are frozen under `legacy/`, still
  built by CI; the legacy APK release pipeline now fires on `v1.*` tags only.
- The test suite is rebuilt for Rust: 77 tests across both crates
  (39 core unit + 13 server unit + 25 HTTP integration driving the real
  router). The suite found and fixed a real bug on day one: axum's built-in
  `Form` extractor (serde_urlencoded) rejects single-value `Vec<String>`
  fields, which would have broken every one-bucket filing; forms now parse
  with `serde_html_form`.

### Why

- The repository is called rusty-box; the app finally matches.
- One small Rust binary is deployable anywhere (a phone via termux, a
  Raspberry Pi, any VPS) without the Android SDK/Gradle/JDK toolchain that
  v1's APK pipeline required.
- The core library is UI-agnostic, which keeps future shells (desktop or
  mobile GUIs reusing the same domain) cheap to add.
- Server-rendered HTML with compile-time template escaping closes the class
  of XSS bugs the jsdom suite found in v1's hand-rolled `escapeHtml`.

### Migration notes

- From the browser app: export JSON (⬇ button), paste it into the new
  import box on the v2 main page, choose merge or replace. Done.
- The Android APK for v2 does not exist; v1 APKs remain installable and
  buildable from `legacy/`. The data inside a v1 install (localStorage) is
  exported through v1's own ⬇ JSON button first.
- Server config: `IDEABUCKET_HOST` (default `127.0.0.1` — there is no auth,
  keep it local), `IDEABUCKET_PORT` (default `8088`),
  `IDEABUCKET_DATA` (default `data/idea-bucket.json`).

### Version

- Workspace version: `2.0.0` (in `Cargo.toml` `[workspace.package]`,
  inherited by both crates).
- No `versionCode` — that is an Android concept and applies to legacy
  releases only.

### Commits

- `4cf08b6` — docs: add pure-Rust rewrite plan for v2.0.0
- `1779d91` — chore: freeze v1 web/Android app under legacy/ and repoint CI
- `2d189ba` — feat: ideabucket-core — v1-compatible domain model, store, exporters, import
- `fd7deeb` — feat: ideabucket-server — zero-JS axum UI, ported v1 design, full HTTP test suite

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
- `51ae2c8` — build: add the v1.2.0 top-3 release commit
- `ff7c16e` — docs: fix the v1.2.0 commit hash in RELEASES.md
- `03bf723` — docs: record ff7c16e in the v1.2.0 release notes
- `80af75a` — docs: record 03bf723 in the v1.2.0 release notes

### Version

- `versionName`: 1.2.0
- `versionCode`: 1 (first real release; there are no previous release tags in this repository, so 1 is the natural starting point). Set at release build time with `-PversionCode=1` — and increase it on every later release.

### Build notes

- Web build: `npm run build`
- Sync into Android: `npm run cap:sync`
- Release APK: `cd android && ./gradlew assembleRelease -PversionName=1.2.0 -PversionCode=1`
- Pre-release gate: `bash check-apk.sh <apk>`

### Release state

The source release is committed on main. The artifact release is not built or signed here. That still needs the toolchain from `build-env.sh`, the chosen `versionCode=1`, and the gate to pass on the machine or CI that has it.

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
- `versionCode`: 1 (first real release; there are no previous release tags in this repository, so 1 is the natural starting point). Set at release build time with `-PversionCode=1` — and increase it on every later release.

### Build notes

- Web build: `npm run build`
- Sync into Android: `npm run cap:sync`
- Release APK: `cd android && ./gradlew assembleRelease -PversionName=1.1.0 -PversionCode=1`
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
- `versionCode`: 1 (first real release; there are no previous release tags in this repository, so 1 is the natural starting point). Set at release build time with `-PversionCode=1` — and increase it on every later release.

### Build notes

- Web build: `npm run build`
- Sync into Android: `npm run cap:sync`
- Release APK: `cd android && ./gradlew assembleRelease -PversionName=1.0.0 -PversionCode=1`
- Pre-release gate: `bash check-apk.sh <apk>`

---

## release-v1.2.0.sh

This is the release script for v1.2.0. It belongs with this release's documentation, not to the changelog sections above.

```bash
#!/usr/bin/env bash
set -euo pipefail

VERSION_NAME="1.2.0"
VERSION_CODE="${VERSION_CODE:-1}"

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
