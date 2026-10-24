#!/usr/bin/env bash
# build-apk.sh — one command to build the "By the way" (راستی) APK, from any fresh shell/login.
#   bash build-apk.sh          → syncs web assets, builds, refreshes by-the-way-debug.apk
#   bash check-apk.sh          → then run the 7-gate pre-release check
set -euo pipefail
cd "$(dirname "$0")"

# Self-contained environment (same pins as check-apk.sh) — survives reboot/logout.
# The dev-machine pins apply only when the calling environment does not already
# provide a toolchain: CI runners export JAVA_HOME (actions/setup-java) and
# ANDROID_HOME (preinstalled SDK), and those must win. Forcing the pins here
# made every CI APK build fail with "JAVA_HOME is set to an invalid directory".
: "${JAVA_HOME:=/usr/lib/jvm/java-21-openjdk}"   # system default JDK breaks AGP
export JAVA_HOME
: "${ANDROID_HOME:=$HOME/Android/Sdk}"
export ANDROID_HOME

# 1. Refresh bundled web assets from the web app (needs node_modules; skip if absent)
if [ -d node_modules ]; then
  npm run cap:sync
else
  echo "!! node_modules missing — run 'npm install' first (cap sync skipped)" >&2
fi

# 2. Build with the Gradle wrapper (downloads nothing from Google; mirrors are prewired)
( cd android && ./gradlew assembleDebug --console=plain )

# 3. Refresh the project-root copy
cp android/app/build/outputs/apk/debug/app-debug.apk by-the-way-debug.apk
echo "✔ by-the-way-debug.apk refreshed — now run: bash check-apk.sh"
