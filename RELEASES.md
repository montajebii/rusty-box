# Releases

Release notes for The Idea Bucket.

## Release format

Every release records:

- Version (`versionName`) and `versionCode`
- What changed
- The commit(s) behind it
- Any build or release-step notes worth keeping

The version is kept consistent across `package.json` and the release build. When a release is cut, `package.json` carries `"version": "<same>"` and the release APK is built with `-PversionName=<same>`. The `versionCode` must increase on every release — Android will not upgrade an installed app otherwise.

## v1.1.0 — 2026-10-18

Dark-theme visual polish across the web app and the Android wrapper.

### What changed

- The whole app got a darker, more deliberate visual pass on both the web and native sides.
- Web (`index.html`):
  - Stable scrollbar gutter so content never jumps when the scrollbar appears (iOS 15+).
  - Thin scrollbars on the idea list, with the macOS overlay suppressed so a native scrollbar does not sit on text.
  - Safe-area insets consumed everywhere they matter: body, bottom bar, scrim, drawer, and the idea detail page.
  - The bucket stage now glows warmly and the liquid surface pulses when there are ideas in view.
  - Blue tap highlight removed from the stage, buttons, cards, and chips.
  - `viewport-fit=cover` + `theme-color` + Apple status-bar style so mobile browsers tint chrome to the dark theme and bleed into the notch area.
  - `color-scheme: dark` on inputs so mobile keyboards render dark chrome around them.
- Android native:
  - Dark status and navigation bars, visible status icons, and `shortEdges` display-cutout handling in the theme.
  - SplashScreen launch theme wired so the warm-up screen is dark instead of the default theme.
  - Adaptive launcher icons: the gold bucket on a dark background (`ic_launcher_foreground`) at hdpi/mdpi/xhdpi/xxhdpi/xxxhdpi, tiled over `@color/ic_launcher_background`.
  - Dark splash screens at every portrait and landscape density, plus a base `drawable/splash.png`.

### Commits

- `dafb3ea` — visual: dark theme polish for web + Android (v1.1.0)

### Version

- `versionName`: 1.1.0
- `versionCode`: set at release build time with `-PversionCode=<n>`; must be higher than the previous release’s code.

### Build notes

- Web build: `npm run build`
- Sync into Android: `npm run cap:sync`
- Release APK: `cd android && ./gradlew assembleRelease -PversionName=1.1.0 -PversionCode=<n>`
- Pre-release gate: `bash check-apk.sh <apk>`

## How to cut the next release

1. Decide the version and version code.
2. Bump `package.json` to that version.
3. Make the changes and commit them.
4. Update this file: add a new section above this one with the version, date, what changed, and the commit(s).
5. Build and run the gate: `npm run build && npx cap sync android`, then `./gradlew assembleRelease -PversionName=<version> -PversionCode=<code>`, then `bash check-apk.sh <apk>`.
6. Ship the APK only after the gate passes.

Do not ship a release without updating this file. This is the record that keeps the version numbers honest.

## Version rule

- `versionName` is the user-facing version and must match `package.json`.
- `versionCode` is an integer that must increase on every release.
- The debug build uses local defaults unless the release step sets them explicitly, so a release must always pass `-PversionName` and `-PversionCode` explicitly.
