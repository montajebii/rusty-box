# The Idea Bucket

Drop a thought in. Dip for old ones.

A minimal idea-capture app: type an idea, drop it into a bucket, and dip back into old ones later. Ideas live in buckets, can be in several at once, and can be searched, edited, exported as JSON or Markdown, and shared. Everything is stored locally in the browser.

The app itself is a single-file web app (`index.html`). The Android APK is a Capacitor wrapper around that web app.

## Version

Current release: **v1.2.0**

The version is tracked in two places and they must agree:

- `package.json` — `"version": "1.2.0"`
- Android release build — passed at build time as `-PversionName=1.2.0 -PversionCode=1`

The Android `build.gradle` does **not** hardcode the release version locally. It reads `-PversionName` / `-PversionCode` from the command line, and only falls back to local defaults (`1.0.0` / `1`) when those are absent. That is deliberate: a release should never accidentally ship with the local default version. The release step is what sets them.

Version code must increase on every release. Android will not replace an installed app with one whose `versionCode` did not go up.

### Release model

This project ships the Android release through the `@capacitor/android` pipeline, and the APK is built on the dev machine or CI where the toolchain from `build-env.sh` exists. The source release is committed here; the artifact release is not built or signed in this workspace.

For a real v1.2.0 release you still need:

1. A `versionCode`. This is the first real release for the repository, so it starts at `1`. Every later release must use a higher number.
2. `npm run build && npx cap sync android` so the APK carries the latest `index.html` and plugin registrations.
3. `cd android && ./gradlew assembleRelease -PversionName=1.2.0 -PversionCode=1`
4. `bash check-apk.sh android/app/build/outputs/apk/release/app-release.apk` — do not ship until that passes.
5. The release notes in `RELEASES.md` must still match the committed state before shipping.

The release script for this release lives in `RELEASES.md` at the bottom. The commit-based release notes live at the top of that file. Do not edit the release script and the changelog independently.

## What is in this repo

- `index.html` — the whole app, in one file
- `package.json` — app metadata, version, and the Capacitor/npm scripts
- `www/` — the web build. `npm run build` regenerates it from `index.html`
- `capacitor.config.json` — Capacitor app id, name, web dir, background color
- `android/` — the Capacitor Android project
- `check-apk.sh` — pre-release gate. Run it on an APK before shipping.
- `build-env.sh` — resolves the Android SDK/JDK/build-tools the build and check scripts need

## Quick start (web)

```bash
npm run build
npm run cap:sync
```

That copies `index.html` into `www/`, then syncs it into the Android assets. The web app can be served directly from `www/` or opened from `index.html` during development.

## Building the Android APK

The Android build needs a real Android toolchain: an Android SDK with `build-tools;35.0.0` and `platforms;android-35`, plus JDK 21 (AGP 8.13 requires it). `build-env.sh` is the one script that decides where those live, and `check-apk.sh` sources it.

```bash
cd android
./gradlew assembleDebug        # debug APK
./gradlew assembleRelease -PversionName=1.1.0 -PversionCode=<n>   # release APK
```

The debug APK lands at `android/app/build/outputs/apk/debug/app-debug.apk`.

## Releases

A release is:

1. The version is agreed on and set in `package.json` and in the release build command.
2. `npm run build && npx cap sync android` so the APK carries the latest `index.html` and plugin registrations.
3. A release build: `./gradlew assembleRelease -PversionName=1.2.0 -PversionCode=1`
4. Run the gate: `bash check-apk.sh android/app/build/outputs/apk/release/app-release.apk` (or the debug APK if you are checking that). It verifies the APK is valid, zipaligned, correctly badged, carries the web assets, has the export plugins registered, has launcher icons, keeps the theme invariant, and verifies the signature.
5. Both the version bump and the rationale are recorded in `RELEASES.md`.

For this release, the release script lives in `RELEASES.md` at the top and documents the exact sequence; run it only on a machine or CI that has the toolchain from `build-env.sh`.

Do not release without updating `RELEASES.md`. The changelog is the record that keeps version numbers honest.

## Pre-release gate

`check-apk.sh` checks, in order:

1. JVM unit tests (Robolectric) pass
2. The APK exists, is a valid zip, and is zipaligned
3. Manifest badging: package, label, min/target SDK, launchable activity, and — when a release states them — the exact `versionName` and `versionCode`
4. Web assets (`assets/public/index.html`) are bundled
5. Export plugins (`@capacitor/filesystem`, `@capacitor/share`) are registered
6. Launcher icon PNGs are packaged
7. The theme invariant: `AppTheme.NoActionBar` must descend from an AppCompat-family theme (the real device crash regression)
8. The APK signature verifies

## Environment for builds

Builds require the toolchain from `build-env.sh`. On machines without it, the build cannot run. This includes the current cloud environment: there is no Android SDK, no JDK, and no `~/Android/Sdk` here, so `assembleDebug` and `assembleRelease` cannot be executed from this workspace. That is expected and does not block anything — the web app, the sync, and the commit are all real; only the Gradle APK compile needs the missing toolchain.

When in doubt about whether a machine can build, run this first:

```bash
. ./build-env.sh && echo "ready" || echo "missing toolchain"
```

## Locales

The app ships its name in Persian too (`راستی`). Keep any locale additions consistent with what `check-apk.sh` expects (it checks for the Persian label in the APK).

## Tests

The web app has a full jsdom test suite (no browser needed) — the real `index.html` is booted in a simulated DOM and driven end to end:

```bash
npm test          # node --test "tests/**/*.test.js"
```

| Suite | What it covers |
|---|---|
| `tests/static.test.js` | structure: inline script parses, every `$('id')` target exists, no duplicate ids, workflows are valid YAML, `bash -n` on all scripts |
| `tests/storage.test.js` | persistence: round-trips, junk filtering, legacy-entry normalization, corrupt-JSON quarantine, quota-failure toast |
| `tests/buckets.test.js` | bucket model: naming rules, case-insensitive reuse, multi-bucket ideas, pending ("file into") flow, bucket deletion |
| `tests/composer.test.js` | capture: drops via button/Enter, empty rejection, glass level/counter/drip, drop animation, source-line arming |
| `tests/drawer.test.js` | ideas drawer: open/close/a11y, search over text and buckets, empty states, filter chips |
| `tests/idea-page.test.js` | idea page: edit flow, draft bucket toggles, close guard (nothing unsaved is silently lost), delete, share fallback, Escape scoping |
| `tests/export.test.js` | JSON + Markdown export payloads (sections, "also in" notes, unfiled) |
| `tests/route.test.js` | deep links `/idea/<id>`, unknown-id bounce |
| `tests/scripts.test.js` | `build-env.sh` toolchain contract, `make-release.sh` semver guard, `check-apk.sh` fail-fast |

The suite runs on every push in the `web` job of the Build workflow. `tests/helpers/boot.js` loads `index.html` with stubs for `confirm`/`prompt`/clipboard/downloads so flows can be driven deterministically.

## Tech

- Web: single-file HTML/CSS/JS, no framework
- Android: Capacitor 8, with `@capacitor/filesystem` and `@capacitor/share` for exports on Android
- Build: Gradle with AGP 8.13, target SDK 35, min SDK 24
