#!/bin/sh
set -eu

[ "${AIRCAST_SKIP_CORE:-}" = 1 ] && exit 0

case "${PLATFORM_NAME:-iphonesimulator}" in
    iphoneos) triple=aarch64-apple-ios ;;
    macosx) triple=aarch64-apple-darwin ;;
    *) triple=aarch64-apple-ios-sim ;;
esac

cargo_bin=$(PATH="$HOME/.cargo/bin:$PATH" && dirname "$(command -v cargo)")

cd "$(dirname "$0")/../groundstation"
exec env -i HOME="$HOME" PATH="$cargo_bin:/usr/bin:/bin:/usr/sbin:/sbin" \
    IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-17.0}" \
    MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-14.0}" \
    cargo build --lib --release --features native-host,synthetic --target "$triple"
