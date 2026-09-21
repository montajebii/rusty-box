# Idea Bucket — Android APK

The web app (`index.html`) is wrapped as a native Android app with [Capacitor](https://capacitorjs.com). No code changes are needed to the web app; it ships as-is and localStorage persists inside the app.

**Current APK:** `idea-bucket-debug.apk` (also at `android/app/build/outputs/apk/debug/app-debug.apk`)
Package: `com.ideabucket.app` · App name: **Idea Bucket** · minSdk 24 (Android 7.0+) · targetSdk 35

---

## ⚠️ TEST BEFORE RELEASE — one command

The 2026-09-22 release **crashed instantly on a Galaxy A52s (Android 14)**. The tests
and checks below exist so that never ships again:

```bash
bash check-apk.sh            # or: bash check-apk.sh path/to/other.apk
```

7 gates, ~30 s: Robolectric unit tests → zip/zipalign → badging (package/label/sdk)
→ web assets bundled → launcher icons → **theme invariant** → signature. It exits
non-zero if anything fails. The unit tests (in `android/app/src/test/`) launch a real
`AppCompatActivity` with the app's actual theme on SDK 28 and 34 — the exact mechanism
that crashed the device build.

Release ritual: `npm run cap:sync` → `cd android && ./gradlew assembleDebug` →
`bash check-apk.sh` → copy `app/build/outputs/apk/debug/app-debug.apk` to the phone.

### Postmortem: the A52s launch crash

The first APK's `styles.xml` had been rewritten with `android:Theme.Material` as the
parent of `AppTheme.NoActionBar`. Capacitor's `BridgeActivity` extends
`AppCompatActivity`, which calls `setTheme(R.style.AppTheme_NoActionBar)` in
`onCreate` — and app-level styles override the library's same-named style. AppCompat
enforces its own theme at startup, so the app died before drawing one frame
(*"You need to use a Theme.AppCompat theme"*). Fixed by restoring the Capacitor
template's `Theme.AppCompat.DayNight.NoActionBar` lineage; pinned by unit test and
by gate 6 of `check-apk.sh`.

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
- Your ideas live in the WebView's localStorage: they survive app updates but are wiped by "Clear app data" / uninstall. Use the in-app **⬇ JSON export** as a backup.

---

## Rebuilding after changes to the web app

```bash
npm run cap:sync        # copies index.html into www/ and syncs to android/
cd android && ./gradlew assembleDebug
bash ../check-apk.sh    # NEVER skip this — it exists because of the A52s crash
cp app/build/outputs/apk/debug/app-debug.apk ../idea-bucket-debug.apk
```
First build after a `--stop` or cache clear is slower; incremental builds take seconds.

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

Debug APKs show an "installed for testing" feel and are larger. For a proper release:

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
