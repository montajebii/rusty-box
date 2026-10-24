# build-env.sh — the one place that decides *where the toolchain lives*.
#
# SOURCED, never executed. build-apk.sh and check-apk.sh both do:
#
#   . ./build-env.sh || exit 1
#
# It exists because those scripts have to run in two very different places:
#
#   1. this project's dev machine, whose network cannot reach dl.google.com — it uses a
#      forged SDK at ~/Android/Sdk and a pinned JDK at /usr/lib/jvm/java-21-openjdk-amd64
#      (see ~/Android/README-SDK-FORGE.md);
#   2. a standard machine or CI runner, where the SDK is wherever ANDROID_HOME points
#      (GitHub's ubuntu-24.04 image: /usr/local/lib/android/sdk, already carrying
#      platforms;android-35 and build-tools;35.0.0) and the JDK is whatever JAVA_HOME
#      provides (that image ships JDK 21 but defaults to 17).
#
# Exports on success:  ANDROID_HOME, ANDROID_SDK_ROOT, JAVA_HOME
# Sets:                SDK, JDK, BUILD_TOOLS, GRADLE_JDK_ARGS
# Returns non-zero, with a message on stderr, when a piece of the toolchain is missing.

# --- Android SDK: an SDK already configured in the environment wins; the dev machine's
#     forged SDK is the fallback.
SDK=""
for candidate in "${ANDROID_HOME:-}" "${ANDROID_SDK_ROOT:-}" "$HOME/Android/Sdk"; do
  if [ -n "$candidate" ] && [ -d "$candidate" ]; then SDK="$candidate"; break; fi
done
if [ -z "$SDK" ]; then
  printf '!! no Android SDK found — set ANDROID_HOME, or install the forged SDK at %s\n' \
    "$HOME/Android/Sdk" >&2
  return 1
fi
export ANDROID_HOME="$SDK"
export ANDROID_SDK_ROOT="$SDK"

# --- JDK: the dev machine's pinned JDK 21 first (its system-default JDK breaks AGP),
#     then whatever JAVA_HOME provides on a standard machine or CI runner.
JDK=""
for candidate in /usr/lib/jvm/java-21-openjdk-amd64 "${JAVA_HOME:-}"; do
  if [ -n "$candidate" ] && [ -x "$candidate/bin/javac" ]; then JDK="$candidate"; break; fi
done
if [ -z "$JDK" ]; then
  printf '!! no JDK found — set JAVA_HOME (checked /usr/lib/jvm/java-21-openjdk-amd64 and %s)\n' \
    "${JAVA_HOME:-<unset>}" >&2
  return 1
fi
export JAVA_HOME="$JDK"

# --- Build tools: 35.0.0 is what AGP 8.13 uses, and what the forged SDK provides.
BUILD_TOOLS="$SDK/build-tools/35.0.0"
if [ ! -x "$BUILD_TOOLS/aapt2" ]; then
  printf '!! Android build-tools 35.0.0 not found — expected %s/aapt2\n' "$BUILD_TOOLS" >&2
  return 1
fi

# --- Why the extra Gradle flag: android/gradle.properties pins org.gradle.java.home to the
#     dev machine's JDK path, and Gradle refuses to start at all when that path does not
#     exist. The command line outranks the file, so every gradlew call passes the JDK
#     resolved above — one path for both machines:
#
#       ( cd android && ./gradlew "${GRADLE_JDK_ARGS[@]}" assembleDebug )
GRADLE_JDK_ARGS=("-Dorg.gradle.java.home=$JDK")
