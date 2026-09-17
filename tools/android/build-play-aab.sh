#!/bin/bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
qt=${QT_ANDROID_ROOT:-$HOME/Qt/6.10.3}
sdk=${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}
ndk=${ANDROID_NDK_ROOT:-$sdk/ndk/27.2.12479018}
build=${BUILD_DIR:-$root/build-android-play}
export JAVA_HOME=${JAVA_HOME:-/opt/homebrew/opt/openjdk@17}
export ANDROID_SDK_ROOT=$sdk ANDROID_NDK_ROOT=$ndk

"$qt/android_arm64_v8a/bin/qt-cmake" -S "$root" -B "$build" -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DQT_HOST_PATH="$qt/macos" \
    -DANDROID_SDK_ROOT="$sdk" -DANDROID_NDK_ROOT="$ndk" \
    -DQGC_QT_MINIMUM_VERSION=6.10.3 -DQGC_QT_MAXIMUM_VERSION=6.10.3 \
    -DQGC_QT_ANDROID_MIN_SDK_VERSION=28 \
    -DQGC_USE_CACHE=OFF
cmake --build "$build" --target aar

export AIRCAST_QGC_AAR="$build/android-build/AircastQGC.aar"
export AIRCAST_APPLICATION_ID=one.aircast.app
export AIRCAST_VERSION_NAME=${AIRCAST_VERSION_NAME:-$(git -C "$root" describe --tags --abbrev=0 | sed 's/^aircast-v//; s/-dev\..*//')}
export AIRCAST_VERSION_CODE=${AIRCAST_VERSION_CODE:?set the Play versionCode}
export AIRCAST_KEYSTORE=${AIRCAST_KEYSTORE:-$HOME/.android/aircast-upload.keystore}
export AIRCAST_KEY_ALIAS=${AIRCAST_KEY_ALIAS:-aircast-upload}
export AIRCAST_KEYSTORE_PASSWORD=${AIRCAST_KEYSTORE_PASSWORD:?keystore password}
(cd "$root/android" && ./gradlew :app:bundleRelease)

aab=$root/android/app/build/outputs/bundle/release/app-release.aab
"$root/tools/android/check-16k.sh" "$aab"
echo "$aab"
