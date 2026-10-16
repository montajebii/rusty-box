#!/usr/bin/env bash
# build-apk.sh — one command to build the Idea Bucket APK, from any fresh shell/login.
#   bash build-apk.sh                        → syncs web assets, builds, refreshes idea-bucket-debug.apk
#   bash build-apk.sh -PversionName=1.1.0 -PversionCode=42
#                                            → extra arguments are passed straight to Gradle
#   bash check-apk.sh                        → then run the 8-gate pre-release check
#
# Works in both places this project is built: the dev machine (forged SDK, pinned JDK) and a
# standard machine or CI runner (ANDROID_HOME/JAVA_HOME from the environment). build-env.sh
# resolves the toolchain for both and fails with a readable message when one is missing.
set -euo pipefail
cd "$(dirname "$0")"

# JDK 21 + Android SDK + the Gradle flag that overrides the dev machine's JDK path.
. ./build-env.sh || exit 1

# 1. Refresh bundled web assets from the web app (needs node_modules; skip if absent)
if [ -d node_modules ]; then
  npm run cap:sync
else
  echo "!! node_modules missing — run 'npm install' first (cap sync skipped)" >&2
fi

# 2. Build with the Gradle wrapper (downloads nothing from Google; mirrors are prewired)
( cd android && ./gradlew "${GRADLE_JDK_ARGS[@]}" assembleDebug --console=plain "$@" )

# 3. Refresh the project-root copy
cp android/app/build/outputs/apk/debug/app-debug.apk idea-bucket-debug.apk
echo "✔ idea-bucket-debug.apk refreshed — now run: bash check-apk.sh"
