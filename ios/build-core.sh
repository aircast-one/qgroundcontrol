#!/bin/sh
set -eu

[ "${AIRCAST_SKIP_CORE:-}" = 1 ] && exit 0

case "${PLATFORM_NAME:-iphonesimulator}" in
    iphoneos) triple=aarch64-apple-ios ;;
    *) triple=aarch64-apple-ios-sim ;;
esac

cd "$(dirname "$0")/../groundstation"
exec env -i HOME="$HOME" PATH="$HOME/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin" \
    IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-17.0}" \
    cargo build --lib --release --features native-host --target "$triple"
