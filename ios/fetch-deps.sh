#!/bin/sh
set -eu

MAPLIBRE_VERSION=6.31.0
MAPLIBRE_SHA256=de3aaa435dd86768b06d90245e630d068dd7eef1491afae7217d1654c52c462a
config="$(dirname "$0")/../.github/build-config.json"
GSTREAMER_VERSION=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["gstreamer"]["version"]["ios"])' "$config")
GSTREAMER_SHA256=$(python3 -c 'import json,sys; c=json.load(open(sys.argv[1]))["gstreamer"]; print(c["checksums"][c["version"]["ios"]]["ios"])' "$config")
CA_BUNDLE_SHA256=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["gstreamer"]["ca_bundle_sha256"])' "$config")

vendor="$(cd "$(dirname "$0")" && pwd)/Vendor"
cache="$HOME/.cache/aircast-ios"
mkdir -p "$vendor" "$cache"

fetch() {
    url=$1 file=$2 sum=$3
    if [ ! -f "$file" ] || ! echo "$sum  $file" | shasum -a 256 -c - >/dev/null 2>&1; then
        curl -fsSL -o "$file.part" "$url"
        echo "$sum  $file.part" | shasum -a 256 -c - >/dev/null
        mv "$file.part" "$file"
    fi
}

if [ ! -f "$vendor/MapLibre-$MAPLIBRE_VERSION" ]; then
    zip="$cache/MapLibre-$MAPLIBRE_VERSION.zip"
    fetch "https://github.com/maplibre/maplibre-native/releases/download/ios-v$MAPLIBRE_VERSION/MapLibre.dynamic.xcframework.zip" "$zip" "$MAPLIBRE_SHA256"
    rm -rf "$vendor/MapLibre.xcframework" "$vendor"/MapLibre-*
    unzip -q "$zip" -d "$vendor"
    touch "$vendor/MapLibre-$MAPLIBRE_VERSION"
fi

fetch "https://curl.se/ca/cacert.pem" "$vendor/ca-certificates.crt" "$CA_BUNDLE_SHA256"

sdk="$HOME/Library/Developer/GStreamer/iPhone.sdk"
if [ ! -f "$sdk/GStreamer-$GSTREAMER_VERSION" ]; then
    pkg="$cache/gstreamer-ios-$GSTREAMER_VERSION.pkg"
    fetch "https://gstreamer.freedesktop.org/data/pkg/ios/$GSTREAMER_VERSION/gstreamer-1.0-devel-$GSTREAMER_VERSION-ios-universal.pkg" "$pkg" "$GSTREAMER_SHA256"
    expanded="$cache/gstreamer-ios-$GSTREAMER_VERSION"
    rm -rf "$expanded"
    pkgutil --expand-full "$pkg" "$expanded"
    mkdir -p "$sdk"
    rm -rf "$sdk/GStreamer.framework" "$sdk"/GStreamer-*
    mv "$expanded"/ios-framework-*/Payload/GStreamer.framework "$sdk/"
    rm -rf "$expanded"
    touch "$sdk/GStreamer-$GSTREAMER_VERSION"
fi
