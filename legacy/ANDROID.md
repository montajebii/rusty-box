# Idea Bucket — Android APK

The web app (`index.html`) is wrapped as a native Android app with [Capacitor](https://capacitorjs.com). No code changes are needed to the web app; it ships as-is and localStorage persists inside the app.

**Current APK:** `idea-bucket-debug.apk` (also at `android/app/build/outputs/apk/debug/app-debug.apk`)
Package: `com.ideabucket.app` · App name: **Idea Bucket** · minSdk 24 (Android 7.0+) · targetSdk 35

---

## Build (reboot-proof, no environment setup needed)

```bash
bash build-apk.sh    # sync + assembleDebug + refresh idea-bucket-debug.apk
bash check-apk.sh    # 8-gate pre-release check (must pass before installing)
```

Both scripts take their toolchain from `build-env.sh`: an `ANDROID_HOME`/`JAVA_HOME` already in
the environment wins, otherwise they fall back to the forged SDK and the pinned JDK here. So
APK builds work in any fresh terminal after a reboot — and the same two commands run unchanged
on a standard machine or a CI runner. `~/.profile` still sources `~/Android/env.sh` on login for
interactive use.

## ⚠️ TEST BEFORE RELEASE — one command

The 2026-09-22 release **crashed instantly on a Galaxy A52s (Android 14)**. The tests
and checks below exist so that never ships again:

```bash
bash check-apk.sh            # or: bash check-apk.sh path/to/other.apk
```

8 gates, ~30 s: Robolectric unit tests → zip/zipalign → badging (package/label/sdk)
→ web assets bundled → **export plugins registered** → launcher icons → **theme invariant**
→ signature. It exits non-zero if anything fails. The unit tests (in `android/app/src/test/`) launch a real
`AppCompatActivity` with the app's actual theme on SDK 28 and 34 — the exact mechanism
that crashed the device build.

Release ritual: `npm run cap:sync` → `cd android && ./gradlew assembleDebug` →
`bash check-apk.sh` → copy `app/build/outputs/apk/debug/app-debug.apk` to the phone.
(`build-apk.sh` does the first two steps and refreshes `idea-bucket-debug.apk`.)

### Postmortem: the A52s launch crash

The first APK's `styles.xml` had been rewritten with `android:Theme.Material` as the
parent of `AppTheme.NoActionBar`. Capacitor's `BridgeActivity` extends
`AppCompatActivity`, which calls `setTheme(R.style.AppTheme_NoActionBar)` in
`onCreate` — and app-level styles override the library's same-named style. AppCompat
enforces its own theme at startup, so the app died before drawing one frame
(*"You need to use a Theme.AppCompat theme"*). Fixed by restoring the Capacitor
template's `Theme.AppCompat.DayNight.NoActionBar` lineage; pinned by unit test and
by gate 7 of `check-apk.sh`.

Second lesson from the same hunt: the forged `android.jar` must keep the official
stub semantics (**every constructor calls `super()` before its `Stub!` throw**), or
AGP's mockable-jar transform silently produces dead constructor bodies and Robolectric
sandboxes poison themselves (`huh? <init>()V`). That lives in the SDK forge
(`~/Android/README-SDK-FORGE.md §4.7`), not in this project.

---

## Install on your phone (sideload)

**Option A — file copy**
1. Copy `idea-bucket-debug.apk` to your phone (USB, email, cloud drive — any way).
2. Tap the file in your file manager. Android will warn about unknown apps:
   allow **"Install unknown apps"** for that file manager when prompted.
3. Install. Done — the Idea Bucket icon (gold bucket) appears in your launcher.

**Option B — adb** (if USB debugging is enabled)
```bash
adb install idea-bucket-debug.apk
```

Notes:
- **Debug APKs are self-signed** and perfectly fine for personal use.
- Your ideas live in the WebView's localStorage: they survive app updates but are wiped by "Clear app data" / uninstall. Use the in-app **⬇ JSON / ⬇ Markdown exports** as a backup.
- On Android, the export buttons write the file and open the system **share sheet**, so you can save it to Files, Drive, Notes, or send it to another app. (A WebView ignores a plain `<a download>` of a blob URL, which is why the app uses the native `@capacitor/filesystem` + `@capacitor/share` plugins. Keep them installed and re-run `npm run cap:sync` — `check-apk.sh` gate 5 fails the release if they are missing.)
- **An idea can be filed into several buckets.** A bucket is only a name, and each idea keeps its own list of them, so nothing is duplicated and old ideas (no `buckets` key) load untouched. The drawer grows a chip per bucket with its count: tap one to filter the list **and** the glass behind it, tap **All** to clear the filter, hold a chip to delete the bucket — the ideas are never deleted with it, they are only unfiled, and the confirmation says so. The **file into** row under the composer picks which bucket(s) the next dropped idea joins, **✏ Edit** on an idea toggles its buckets (Cancel discards those toggles like any other unsaved change), and the idea page plus every card show the memberships as chips. Search matches bucket names as well as idea text, and the Markdown export gets one `##` section per bucket, listing an idea under each bucket it is in with an “also in …” note.
- **Every idea is editable.** Tap **✏** on a card (or open an idea and press **✏ Edit**) to switch the detail page into an inline editor: Save / Cancel, a live character counter, 2000-character limit, and `Ctrl/⌘+Enter` to save from a hardware keyboard. Saved edits keep the idea's `id`/`color`/`created` and add an `edited` timestamp, shown as “Edited …” on the detail page and “· edited” in the list. An empty idea can't be saved, and closing with unsaved changes asks first. Closing the editor blurs the field so the soft keyboard goes away with it.

---

## Rebuilding after changes to the web app

```bash
npm run cap:sync        # copies index.html into www/ and syncs to android/
cd android && ./gradlew assembleDebug
bash ../check-apk.sh    # NEVER skip this — it exists because of the A52s crash
cp app/build/outputs/apk/debug/app-debug.apk ../idea-bucket-debug.apk
```

`index.html` is the single source of truth — all UI (including the idea editor) lives in
that one file, so there is nothing to rebuild beyond `cap:sync`.
First build after a `--stop` or cache clear is slower; incremental builds take seconds.

---

## CI: a build on every commit, a release on every tag

Two GitHub Actions workflows in `.github/workflows/` build on GitHub's runners, so a change is
checked without any local toolchain — and without a phone — in between.

**`build.yml` — every push to any branch, every PR** (Actions → Build → the run)

1. `web` job: `npm ci`, then a parse check of the app's inline `<script>`. `index.html` is the
   whole app, so one stray brace there white-screens it on launch and nothing else would notice.
2. `apk` job: `cap sync` → `assembleDebug` → **the same 8-gate `check-apk.sh` a release must
   pass** → uploads `idea-bucket-debug` as a build artifact. Download it from the run summary
   and sideload it.

**`release.yml` — every `v*` tag**, or run it by hand with the name of an existing tag:

```bash
git tag v1.1.0
git push origin v1.1.0     # → Build + publish → a GitHub Release with the APK
```

It builds the release APK, runs the same 8 gates, and publishes the APK with a `SHA-256`
(`sha256sum -c idea-bucket-v1.1.0.apk.sha256`). `versionName` comes from the tag and
`versionCode` from the workflow run number, so each release installs over the previous one.
A re-run replaces the release's assets instead of failing.

Signing is optional but recommended. Add these repository secrets (*Settings → Secrets and
variables → Actions*) and the release is signed with your own key:

| Secret | Value |
|---|---|
| `KEYSTORE_BASE64` | `base64 -w0 ~/idea-bucket.keystore` |
| `KEYSTORE_PASSWORD` | the keystore password |
| `KEY_ALIAS` | the key alias (`ideabucket` in the commands below) |
| `KEY_PASSWORD` | the key password |

Without them the release falls back to the self-signed debug APK and says so in its notes: it
installs, but its signature differs, so it cannot upgrade an install signed with your own key.

Notes on the setup: both workflows pin `runs-on: ubuntu-24.04` (the `ubuntu-latest` label moves
to Ubuntu 26.04 in November 2026) and select the runner's JDK 21, because
`android/gradle.properties` pins the JDK path of *this* machine. CI runs the Robolectric JVM
tests; the instrumented tests in `android/app/src/androidTest/` still need a real device, and no
workflow touches your phone.

---

## Toolchain on this machine (already set up — shared by future projects)

This machine **cannot reach Google's artifact servers** (`dl.google.com` Android paths are
filtered), so the environment is provisioned with mirrors and forged SDK packages.

**📖 The full system documentation lives outside this project, for reuse by any future
native Android app:**

- `~/Android/README-SDK-FORGE.md` — how the SDK is forged, exact schemas, error cookbook
- `~/Android/forge-android-sdk.sh` — recreates the entire SDK non-interactively (idempotent)
- `~/Android/env.sh` — source it: sets `ANDROID_HOME`, `JAVA_HOME` (JDK 21), PATH

Summary of what exists:

| Component | Location | How it was made |
|---|---|---|
| JDK (pinned) | `/usr/lib/jvm/java-21-openjdk-amd64` | Set via `org.gradle.java.home` in `android/gradle.properties` |
| SDK dir | `~/Android/Sdk` | `sdk.dir` in `android/local.properties` |
| Platform `android-35` | `~/Android/Sdk/platforms/android-35` | `android.jar` = Robolectric `android-all` 15 (Maven via Aliyun); hand-written `package.xml` (namespace `repository2/03`, type `platformDetailsType`), `source.properties` (`AndroidVersion.ApiLevel=35`), `build.prop` |
| Build tools `35.0.0` | `~/Android/Sdk/build-tools/35.0.0` | Copied from Debian's `29.0.3`, `Pkg.Revision` bumped to 35.0.0; missing binaries stubbed; `core-for-system-modules.jar` forged from JDK 21's `java.base.jmod` classes |
| Google Maven | `maven.aliyun.com/repository/google` | Listed **first** in `android/build.gradle` (buildscript + allprojects + subprojects); `google()` kept as fallback |

All of the above is reproducible with one command: `bash ~/Android/forge-android-sdk.sh`.

If you ever run this project on an unrestricted network, you can delete the forged
packages and let the SDK Manager download the real ones (`sdkmanager "platforms;android-35" "build-tools;35.0.0"`).

---

## Release build (signed, recommended for daily use)

Pushing a `v*` tag does all of this for you (see CI above, including the four secrets that make
the artifact signed). This is the same thing by hand — Debug APKs show an "installed for
testing" feel and are larger, so for a proper release:

```bash
# 1. one-time: create a signing key (keep this file + password safe!)
keytool -genkey -v -keystore ~/idea-bucket.keystore -alias ideabucket \
  -keyalg RSA -keysize 2048 -validity 10000

# 2. build unsigned release
cd android && ./gradlew assembleRelease

# 3. align + sign
~/Android/Sdk/build-tools/35.0.0/zipalign -f 4 \
  app/build/outputs/apk/release/app-release-unsigned.apk /tmp/ib-aligned.apk
~/Android/Sdk/build-tools/35.0.0/apksigner sign \
  --ks ~/idea-bucket.keystore --ks-key-alias ideabucket --out ../idea-bucket.apk \
  /tmp/ib-aligned.apk
```

Keep the keystore: updates must be signed with the same key or Android will refuse to upgrade the app.
