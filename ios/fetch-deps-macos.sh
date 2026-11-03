#!/bin/sh
# Dependencies for the macOS build of the app (the AircastMac target), on top of the shared ones
# fetch-deps.sh brings (CA bundle, Cesium).
#
# MapLibre: maplibre-native no longer publishes macOS binaries, but its macOS SDK is still built in
# its own CI from platform/macos, sharing platform/darwin with the iOS SDK. Building it from the
# same tag as the iOS SDK keeps the MLN API the two platforms compile against identical.
#
# GStreamer: the same runtime and devel .pkg files tools/setup/install_dependencies installs into
# /Library/Frameworks for the desktop build, checked against build-config.json.
set -eu

MAPLIBRE_VERSION=6.31.0
here="$(cd "$(dirname "$0")" && pwd)"
config="$here/../.github/build-config.json"
vendor="$here/Vendor/macos"
cache="$HOME/.cache/aircast-macos"
mkdir -p "$vendor" "$cache"

"$here/fetch-deps.sh" --shared-only

if [ ! -f "$vendor/MapLibre-$MAPLIBRE_VERSION" ]; then
    command -v bazelisk >/dev/null 2>&1 || command -v bazel >/dev/null 2>&1 || {
        echo "fetch-deps-macos: building MapLibre for macOS needs bazelisk (brew install bazelisk)" >&2
        exit 1
    }
    bazel=$(command -v bazelisk || command -v bazel)
    src="$cache/maplibre-native-$MAPLIBRE_VERSION"
    if [ ! -d "$src/.git" ]; then
        rm -rf "$src"
        git clone --quiet --depth 1 --branch "ios-v$MAPLIBRE_VERSION" \
            --recurse-submodules --shallow-submodules \
            https://github.com/maplibre/maplibre-native "$src"
    fi
    (cd "$src" && "$bazel" build //platform/macos:MapLibre.dynamic \
        --//:renderer=metal --//platform/macos:macos_loop=cfrunloop --compilation_mode=opt)
    rm -rf "$vendor/MapLibre.xcframework" "$vendor"/MapLibre-*
    unzip -q -o "$src/bazel-bin/platform/macos/MapLibre.dynamic.xcframework.zip" -d "$vendor"
    touch "$vendor/MapLibre-$MAPLIBRE_VERSION"
fi

# Bazel lays the macOS framework out shallow, the way iOS frameworks are, and Xcode will not embed a
# shallow framework in a macOS app ("expected Versions/Current/Resources/Info.plist"). Move each slice
# into the versioned layout macOS expects. Safe to run again on a framework that is already deep.
for framework in "$vendor"/MapLibre.xcframework/*/MapLibre.framework; do
    [ -f "$framework/Info.plist" ] || continue
    version="$framework/Versions/A"
    mkdir -p "$version/Resources"
    for item in "$framework"/*; do
        name=$(basename "$item")
        case $name in
            Versions) ;;
            Info.plist) mv "$item" "$version/Resources/Info.plist" ;;
            MapLibre|Headers|Modules) mv "$item" "$version/$name" ;;
            *) mv "$item" "$version/Resources/$name" ;;
        esac
    done
    ln -sfn A "$framework/Versions/Current"
    for name in MapLibre Headers Modules Resources; do
        [ -e "$version/$name" ] && ln -sfn "Versions/Current/$name" "$framework/$name"
    done
done

gst_version=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["gstreamer"]["version"]["macos"])' "$config")
gst_root=/Library/Frameworks/GStreamer.framework
if [ ! -f "$gst_root/Versions/1.0/lib/pkgconfig/gstreamer-1.0.pc" ] || \
   ! grep -q "Version: $gst_version" "$gst_root/Versions/1.0/lib/pkgconfig/gstreamer-1.0.pc"; then
    base="https://qgroundcontrol.s3.us-west-2.amazonaws.com/dependencies/gstreamer/macos"
    for kind in runtime devel; do
        case $kind in
            runtime) name="gstreamer-1.0-$gst_version-universal.pkg" key=macos ;;
            devel) name="gstreamer-1.0-devel-$gst_version-universal.pkg" key=macos_devel ;;
        esac
        sum=$(python3 -c 'import json,sys; c=json.load(open(sys.argv[1]))["gstreamer"]; print(c["checksums"][c["version"]["macos"]][sys.argv[2]])' "$config" "$key")
        pkg="$cache/$name"
        if [ ! -f "$pkg" ] || ! echo "$sum  $pkg" | shasum -a 256 -c - >/dev/null 2>&1; then
            curl -fsSL -o "$pkg.part" "$base/$name"
            echo "$sum  $pkg.part" | shasum -a 256 -c - >/dev/null
            mv "$pkg.part" "$pkg"
        fi
        sudo installer -pkg "$pkg" -target /
    done
fi
