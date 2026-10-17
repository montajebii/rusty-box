#!/usr/bin/env bash
# check-apk.sh — pre-release gate for the Idea Bucket APK (and a template for future projects).
#
# Run BEFORE handing an APK to a phone. Fails fast with a clear message on every class of
# regression that has actually bitten this project:
#   1. JVM unit tests (Robolectric) must pass        — catches theme/startup regressions
#   2. APK must exist, be a valid zip + zipaligned   — catches broken/missing artifacts
#   3. Manifest badging: package, label, min/target  — catches packaging drift, and the
#      version a release states (EXPECT_VERSION_NAME / EXPECT_VERSION_CODE) so a release
#      can never ship with the local default version by accident
#   4. Web assets (www/index.html) must be inside    — catches a missed `cap sync`
#   5. Export plugins must be registered             — catches a missed `cap sync` that
#      silently breaks the ⬇ JSON / ⬇ Markdown exports (they need @capacitor/filesystem
#      and @capacitor/share on Android; a blob <a download> does nothing in the WebView)
#   6. Launcher icons must be packaged               — catches resource surprises
#   7. Theme invariant: AppTheme.NoActionBar parent must be an AppCompat descendant
#      (AppCompatActivity throws at startup otherwise — this was the real device crash)
#   8. APK signature must be verifiable              — Android refuses unsigned/broken APKs
#
# Usage:  bash check-apk.sh [path/to/app-debug.apk]
#         (default: android/app/build/outputs/apk/debug/app-debug.apk)
#
# The JDK/SDK/build-tools it uses come from build-env.sh, so this runs unchanged on the dev
# machine (forged SDK at ~/Android/Sdk) and on a plain machine or CI runner (ANDROID_HOME).
set -u
cd "$(dirname "$0")"

. ./build-env.sh || exit 1

# Tools these gates need from the host itself. Checked up front so a missing one is named
# here rather than surfacing later as a misleading "corrupt zip" or "wrong package".
for tool in unzip python3; do
  command -v "$tool" >/dev/null || { printf '!! %s is required by this check but is not on PATH\n' "$tool" >&2; exit 1; }
done

APK="${1:-android/app/build/outputs/apk/debug/app-debug.apk}"
BT="$BUILD_TOOLS"
FAIL=0
CURRENT_STEP=""
FAILED=()   # in gate order, for the CI annotation at the end
step() { CURRENT_STEP="$1"; printf '\n== %s ==\n' "$1"; }
ok()   { printf '  \033[32mOK\033[0m  %s\n' "$1"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$1"; FAIL=1; FAILED+=("[$CURRENT_STEP] $1"); }

step "1/8 Unit tests (Robolectric)"
if (cd android && ./gradlew -q "${GRADLE_JDK_ARGS[@]}" :app:testDebugUnitTest) \
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

step "2/8 Artifact & alignment"
if [ ! -f "$APK" ]; then bad "APK not found: $APK (build it first: cd android && ./gradlew assembleDebug)"; else
  ok "exists ($(du -h "$APK" | cut -f1))"
  if unzip -t "$APK" >/dev/null 2>&1; then ok "valid zip"; else bad "corrupt zip"; fi
  if "$BT/zipalign" -c 4 "$APK" >/dev/null 2>&1; then ok "zipaligned"; else bad "NOT zipaligned (run: zipalign -f 4 in.apk out.apk)"; fi
fi

step "3/8 Manifest badging"
BADGE=$("$BT/aapt2" dump badging "$APK" 2>/dev/null) || BADGE=""
# Read the values out first, so a failure reports what the APK actually says instead of only
# "drifted" — on CI the gate message is the only thing visible without a signed-in session.
PKG=$(grep -oE "^package: name='[^']*'" <<<"$BADGE" | head -1)
LABEL=$(grep -oE "^application-label:'[^']*'" <<<"$BADGE" | head -1)
# The minimum-SDK line is spelled `sdkVersion:` by aapt and `minSdkVersion:` by the aapt2 in
# build-tools 35 that CI uses; the value is what this gate is about, so both spellings count.
MINSDK_RE="^(sdkVersion|minSdkVersion):'24'"
MINSDK=$(grep -oE "^(sdkVersion|minSdkVersion):'[^']*'" <<<"$BADGE" | head -1)
TARGETSDK=$(grep -oE "^targetSdkVersion:'[^']*'" <<<"$BADGE" | head -1)
grep -q "^package: name='com.ideabucket.app'" <<<"$BADGE" && ok "package com.ideabucket.app" \
  || bad "wrong/missing package — badging says ${PKG:-<no package line>}"
grep -qE "application-label:['\"](By the way|راستی|Idea Bucket)['\"]" <<<"$BADGE" && ok "label matches" \
  || bad "wrong/missing label — badging says ${LABEL:-<no application-label line>}"
grep -q "application-label-fa:" <<<"$BADGE" && ok "Persian label packaged (values-fa)" \
  || bad "no fa label — values-fa/strings.xml missing from the APK"
grep -qE "$MINSDK_RE" <<<"$BADGE" && ok "minSdk 24" \
  || bad "minSdk drifted — saw ${MINSDK:-<no minSdk line>}"
grep -q "targetSdkVersion:'35'" <<<"$BADGE" && ok "targetSdk 35" \
  || bad "targetSdk drifted — saw ${TARGETSDK:-<no targetSdkVersion line>}"
grep -q "launchable-activity" <<<"$BADGE" && ok "launchable activity present" || bad "no LAUNCHER activity"
# A release states the version it must carry (release.yml passes these). If the APK still
# reports the local defaults, the -PversionName/-PversionCode plumbing did nothing.
if [ -n "${EXPECT_VERSION_NAME:-}" ]; then
  grep -q "versionName='$EXPECT_VERSION_NAME'" <<<"$BADGE" && ok "versionName $EXPECT_VERSION_NAME" \
    || bad "versionName is not $EXPECT_VERSION_NAME — check the -PversionName plumbing"
fi
if [ -n "${EXPECT_VERSION_CODE:-}" ]; then
  grep -q "versionCode='$EXPECT_VERSION_CODE'" <<<"$BADGE" && ok "versionCode $EXPECT_VERSION_CODE" \
    || bad "versionCode is not $EXPECT_VERSION_CODE — check the -PversionCode plumbing"
fi

step "4/8 Web assets bundled"
if unzip -l "$APK" | grep -q "assets/public/index.html"; then ok "assets/public/index.html present"; else bad "index.html missing — run: npm run cap:sync, then rebuild"; fi

step "5/8 Export plugins registered"
PLUGINS=$(unzip -p "$APK" assets/capacitor.plugins.json 2>/dev/null) || PLUGINS=""
if grep -q "@capacitor/filesystem" <<<"$PLUGINS"; then ok "@capacitor/filesystem registered"; else bad "@capacitor/filesystem missing — JSON/Markdown export will not work. run: npm run cap:sync, then rebuild"; fi
if grep -q "@capacitor/share" <<<"$PLUGINS"; then ok "@capacitor/share registered"; else bad "@capacitor/share missing — JSON/Markdown export will not work. run: npm run cap:sync, then rebuild"; fi

step "6/8 Launcher icons"
N_ICONS=$(unzip -l "$APK" | grep -c "res/.*ic_launcher.*\.png")
[ "$N_ICONS" -ge 4 ] && ok "$N_ICONS launcher icon PNGs" || bad "only $N_ICONS icon PNGs"

step "7/8 Theme invariant (the device-crash regression)"
PARENT=$("$BT/aapt2" dump resources "$APK" 2>/dev/null \
  | grep -A1 "style/AppTheme.NoActionBar$" | grep -o "parent=style/[^ ]*" | head -1)
case "$PARENT" in
  parent=style/Theme.AppCompat*) ok "AppTheme.NoActionBar -> $PARENT" ;;
  parent=style/Theme.MaterialComponents*) ok "AppTheme.NoActionBar -> $PARENT (MaterialComponents extends AppCompat)" ;;
  parent=style/Theme.Material3*) ok "AppTheme.NoActionBar -> $PARENT (Material3 extends MaterialComponents -> AppCompat)" ;;
  "") bad "AppTheme.NoActionBar not found or has no parent" ;;
  *)  bad "AppTheme.NoActionBar parent is '$PARENT' — must be Theme.AppCompat.* or Theme.MaterialComponents.* (AppCompatActivity crashes with framework themes)" ;;
esac

step "8/8 Signature"
if "$BT/apksigner" verify "$APK" >/dev/null 2>&1; then ok "signature verifies"; else bad "signature does NOT verify (Android will refuse to install)"; fi

printf '\n'
if [ "$FAIL" -eq 0 ]; then
  printf '\033[32m✔ PRE-RELEASE CHECK PASSED — %s is ready to install.\033[0m\n' "$(basename "$APK")"
else
  printf '\033[31m✘ PRE-RELEASE CHECK FAILED — do not ship this APK.\033[0m\n'
  # On CI, publish the failures as an annotation. A public repository's job log needs a
  # signed-in session to read, while annotations are readable by anyone (and appear on the
  # commit's checks view) — so a red build names the gate that broke without a login.
  if [ -n "${GITHUB_ACTIONS:-}" ]; then
    printf '::error title=check-apk.sh: %s gate failure(s)::%s\n' \
      "${#FAILED[@]}" "$(printf '%s | ' "${FAILED[@]}")"
  fi
fi
exit $FAIL
