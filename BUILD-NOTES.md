# Build notes for v1.2.0

## What built here

- `npm run build` — succeeded. `www/index.html` is fresh from `index.html`.
- `npm run cap:sync` — succeeded. Web assets and Capacitor plugins are synced into `android/`.
- The top 3 ideas from the brainstorming round are in `index.html`:
  - Instant capture entry point (`/`, `+ capture`, composer focus, and a visible source line when text is selected on the page).
  - Inbox-first model: new ideas are valid without a bucket, and the composer explicitly says `no bucket yet — ideas live in the inbox until you file them`.
  - Source-aware ideas: selected text at capture time is kept and shown on the idea page under `from:` when present.
- Bucket assignment is optional at capture time, and the idea page says `In the inbox — no bucket yet` instead of treating that as missing data.

## What did not build here

The Android APK builds (`assembleDebug`, `assembleRelease`) did not run here, and cannot run here.

### Why

`build-env.sh` is the script that decides where the Android toolchain lives. It needs three things:

- An Android SDK with `build-tools;35.0.0` (aapt2, zipalign, apksigner) and `platforms;android-35`
- JDK 21 (AGP 8.13 requires it)
- `ANDROID_HOME` / `ANDROID_SDK_ROOT`, or `~/Android/Sdk` on the dev machine

This cloud environment has none of those: no `ANDROID_HOME`, no `JAVA_HOME`, no `~/Android/Sdk`, and no JDK on `PATH`. `build-env.sh` fails fast with a clear message in that case, which is the correct behavior — it does not silently proceed.

### Where builds do run

- The project’s dev machine, which uses the forged SDK at `~/Android/Sdk` and a pinned JDK at `/usr/lib/jvm/java-21-openjdk-amd64` (see `~/Android/README-SDK-FORGE.md`).
- A standard machine or CI runner, where `ANDROID_HOME` points at a real SDK (for example GitHub’s `ubuntu-24.04` image: `/usr/local/lib/android/sdk`, which already carries `platforms;android-35` and `build-tools;35.0.0`) and `JAVA_HOME` provides JDK 21.

### The release build command

```bash
cd android
./gradlew assembleRelease -PversionName=1.2.0 -PversionCode=1
```

The debug build is:

```bash
cd android
./gradlew assembleDebug
```

The debug APK lands at `android/app/build/outputs/apk/debug/app-debug.apk`.

## Version code

This is the first real release for the repository — there are no previous release tags — so the `versionCode` starts at `1`. It is set explicitly at release build time with `-PversionCode=1`. The `build.gradle` reads it from `-PversionCode` and only falls back to the local default (`1`) when it is absent — that fallback is for development, not for shipping.

Every later release must use a higher `versionCode`, or Android will refuse to upgrade an installed copy.

## The gate

Before any APK is handed to a phone, run:

```bash
bash check-apk.sh <apk>
```

It checks the artifact, the manifest badging (including the version the release claimed), the bundled web assets, the export plugins, the launcher icons, the theme invariant, and the signature. A release does not ship unless that passes.

## What this means for v1.2.0

The work for this release is real and committed. The APK itself still needs to be built on the dev machine or CI, with `-PversionName=1.2.0 -PversionCode=1`, and then passed through `check-apk.sh`.

For this release, the `versionCode` is `1`. Every later release must use a higher `versionCode`, then run the release command with both `-PversionName=<next>` and `-PversionCode=<higher number>`. Do not ship with the local fallback versionCode.

For this release, set it before the APK build. The release checklist in `RELEASES.md` includes it as a required step.

### Release state

The source release is committed on main. The artifact release is not built or signed here. That still needs the toolchain from `build-env.sh`, the chosen `versionCode=1`, and the gate to pass on the machine or CI that has it.
