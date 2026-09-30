#!/bin/bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)

export AIRCAST_VERSION_NAME=${AIRCAST_VERSION_NAME:-$(git -C "$root" describe --tags --abbrev=0 | sed 's/^aircast-v//; s/-dev\..*//')}
export AIRCAST_VERSION_CODE=${AIRCAST_VERSION_CODE:?set the Play versionCode}
export AIRCAST_KEYSTORE=${AIRCAST_KEYSTORE:-$HOME/.android/aircast-upload.keystore}
export AIRCAST_KEY_ALIAS=${AIRCAST_KEY_ALIAS:-aircast-upload}
export AIRCAST_KEYSTORE_PASSWORD=${AIRCAST_KEYSTORE_PASSWORD:?keystore password}

QGC_CONFIGURE_ARGS="-DQGC_USE_CACHE=OFF ${QGC_CONFIGURE_ARGS:-}" \
    QGC_CORE_BUILD_DIR=${BUILD_DIR:-build-android-play} "$root/tools/android/studio.py"

"$root/tools/android/gradle.sh" :app:bundleQtRelease

aab=$root/android/app/build/outputs/bundle/qtRelease/app-qt-release.aab
"$root/tools/android/check-16k.sh" "$aab"
echo "$aab"
