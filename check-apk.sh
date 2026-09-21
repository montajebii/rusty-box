#!/usr/bin/env bash
# check-apk.sh — pre-release gate for the Idea Bucket APK (and a template for future projects).
#
# Run BEFORE handing an APK to a phone. Fails fast with a clear message on every class of
# regression that has actually bitten this project:
#   1. JVM unit tests (Robolectric) must pass        — catches theme/startup regressions
#   2. APK must exist, be a valid zip + zipaligned   — catches broken/missing artifacts
#   3. Manifest badging: package, label, min/target  — catches packaging drift
#   4. Web assets (www/index.html) must be inside    — catches a missed `cap sync`
#   5. Launcher icons must be packaged               — catches resource surprises
#   6. Theme invariant: AppTheme.NoActionBar parent must be an AppCompat descendant
#      (AppCompatActivity throws at startup otherwise — this was the real device crash)
#   7. APK signature must be verifiable              — Android refuses unsigned/broken APKs
#
# Usage:  bash check-apk.sh [path/to/app-debug.apk]
#         (default: android/app/build/outputs/apk/debug/app-debug.apk)
set -u
cd "$(dirname "$0")"

APK="${1:-android/app/build/outputs/apk/debug/app-debug.apk}"
BT="$HOME/Android/Sdk/build-tools/35.0.0"
FAIL=0
step() { printf '\n== %s ==\n' "$1"; }
ok()   { printf '  \033[32mOK\033[0m  %s\n' "$1"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$1"; FAIL=1; }

step "1/7 Unit tests (Robolectric)"
if (cd android && JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64 ./gradlew -q :app:testDebugUnitTest) \
   && python3 - <<'PY'
import glob, xml.dom.minidom, sys
bad = 0; tot = 0
for p in glob.glob('android/app/build/test-results/testDebugUnitTest/TEST-*.xml'):
    r = xml.dom.minidom.parse(p).documentElement
    tot += int(r.getAttribute('tests'))
    bad += int(r.getAttribute('failures')) + int(r.getAttribute('errors')) + int(r.getAttribute('skipped'))
print(f'  {tot} tests, {bad} failed/skipped')
sys.exit(1 if (bad or tot == 0) else 0)
PY
then ok "unit tests green"; else bad "unit tests failing — fix before release"; fi

step "2/7 Artifact & alignment"
if [ ! -f "$APK" ]; then bad "APK not found: $APK (build it first: cd android && ./gradlew assembleDebug)"; else
  ok "exists ($(du -h "$APK" | cut -f1))"
  if unzip -t "$APK" >/dev/null 2>&1; then ok "valid zip"; else bad "corrupt zip"; fi
  if "$BT/zipalign" -c 4 "$APK" >/dev/null 2>&1; then ok "zipaligned"; else bad "NOT zipaligned (run: zipalign -f 4 in.apk out.apk)"; fi
fi

step "3/7 Manifest badging"
BADGE=$("$BT/aapt2" dump badging "$APK" 2>/dev/null) || BADGE=""
grep -q "^package: name='com.ideabucket.app'" <<<"$BADGE" && ok "package com.ideabucket.app" || bad "wrong/missing package"
grep -qE "application-label:['\"]Idea Bucket['\"]" <<<"$BADGE" && ok "label 'Idea Bucket'" || bad "wrong/missing label"
grep -q "sdkVersion:'24'" <<<"$BADGE" && ok "minSdk 24" || bad "minSdk drifted"
grep -q "targetSdkVersion:'35'" <<<"$BADGE" && ok "targetSdk 35" || bad "targetSdk drifted"
grep -q "launchable-activity" <<<"$BADGE" && ok "launchable activity present" || bad "no LAUNCHER activity"

step "4/7 Web assets bundled"
if unzip -l "$APK" | grep -q "assets/public/index.html"; then ok "assets/public/index.html present"; else bad "index.html missing — run: npm run cap:sync, then rebuild"; fi

step "5/7 Launcher icons"
N_ICONS=$(unzip -l "$APK" | grep -c "res/.*ic_launcher.*\.png")
[ "$N_ICONS" -ge 4 ] && ok "$N_ICONS launcher icon PNGs" || bad "only $N_ICONS icon PNGs"

step "6/7 Theme invariant (the device-crash regression)"
PARENT=$("$BT/aapt2" dump resources "$APK" 2>/dev/null \
  | grep -A1 "style/AppTheme.NoActionBar$" | grep -o "parent=style/[^ ]*" | head -1)
case "$PARENT" in
  parent=style/Theme.AppCompat*) ok "AppTheme.NoActionBar -> $PARENT" ;;
  "") bad "AppTheme.NoActionBar not found or has no parent" ;;
  *)  bad "AppTheme.NoActionBar parent is '$PARENT' — must be Theme.AppCompat.* (AppCompatActivity crashes otherwise)" ;;
esac

step "7/7 Signature"
if "$BT/apksigner" verify "$APK" >/dev/null 2>&1; then ok "signature verifies"; else bad "signature does NOT verify (Android will refuse to install)"; fi

printf '\n'
if [ "$FAIL" -eq 0 ]; then
  printf '\033[32m✔ PRE-RELEASE CHECK PASSED — %s is ready to install.\033[0m\n' "$(basename "$APK")"
else
  printf '\033[31m✘ PRE-RELEASE CHECK FAILED — do not ship this APK.\033[0m\n'
fi
exit $FAIL
