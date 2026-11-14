#!/usr/bin/env bash
# Build the arm64-v8a release APK end to end.
#
# Requirements:
#   - Rust toolchain with the aarch64-linux-android target
#   - cargo-ndk              (installed automatically when missing)
#   - Android SDK with platform 35 + build-tools 35.0.0
#   - Android NDK r27+        (export ANDROID_NDK_HOME)
#   - JDK 17                  (JAVA_HOME)
#
# Env:
#   ANDROID_NDK_HOME   required, path to the NDK
#   ANDROID_SDK_ROOT   required for Gradle (or ANDROID_HOME)
#   SKIP_RUST=1        reuse an existing jniLibs .so
#
# Signing (optional): KEYSTORE_PATH, KEYSTORE_PASSWORD, KEY_ALIAS,
# KEY_PASSWORD. Without them the release APK is debug-signed (see
# app/build.gradle.kts), which is fine for local size/behaviour checks.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP="$ROOT/android-app"
JNI_DIR="$APP/app/src/main/jniLibs"

: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to the Android NDK path}"
export ANDROID_NDK_ROOT="${ANDROID_NDK_ROOT:-$ANDROID_NDK_HOME}"
export ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"

if [[ "${SKIP_RUST:-0}" != "1" ]]; then
  echo "==> cargo ndk build (arm64-v8a, --features android)"
  command -v cargo-ndk >/dev/null 2>&1 || cargo install cargo-ndk --locked
  rustup target list --installed | grep -qx aarch64-linux-android \
    || rustup target add aarch64-linux-android
  mkdir -p "$JNI_DIR"
  (
    cd "$ROOT"
    # NOTE: --features belongs to cargo, so it must follow the `build` subcommand.
    # API 26 is the floor: cpal's AAudio backend links libaaudio.so, added in 26.
    cargo ndk -P 26 -t arm64-v8a -o "$JNI_DIR" build --release --lib --features android
  )
  ls -lh "$JNI_DIR/arm64-v8a/libayshamusic.so"
else
  echo "==> SKIP_RUST=1: reusing $JNI_DIR"
fi

echo "==> Gradle assembleRelease"
cd "$APP"
if [[ -x ./gradlew ]]; then
  ./gradlew --no-daemon assembleRelease
elif command -v gradle >/dev/null 2>&1; then
  # Generate the wrapper once, then use it so local and CI builds match.
  # AGP 8.5 requires Gradle 8.7+.
  [[ -f gradle/wrapper/gradle-wrapper.jar ]] || gradle wrapper --gradle-version 8.9
  ./gradlew --no-daemon assembleRelease
else
  echo "error: neither ./gradlew nor gradle is available" >&2
  exit 1
fi

APK_DIR="$APP/app/build/outputs/apk/release"
APK="$(find "$APK_DIR" -maxdepth 1 -name '*.apk' | head -n1)"
[[ -n "$APK" ]] || { echo "error: no APK produced in $APK_DIR" >&2; exit 1; }

echo
echo "==> Built: $APK"
ls -lh "$APK"
"$APP/scripts/measure_apk_size.sh" "$APK"
