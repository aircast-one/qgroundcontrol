#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
SRC=resources/icons/aircast-icon.svg
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

render() { rsvg-convert -w "$2" -h "$2" "$1" -o "$3"; }
bleed() {
  sed -e 's|<rect x="100" y="100" width="824" height="824" rx="186"|<rect x="0" y="0" width="1024" height="1024" rx="'"$1"'"|' \
      -e '/stroke="#48484a"/d' \
      -e 's|<circle cx="512" cy="512" r="290"|<g transform="translate(512 512) scale(1.2427) translate(-512 -512)"><circle cx="512" cy="512" r="290"|' \
      -e 's|</svg>|</g></svg>|' "$SRC"
}
bleed 0 > "$TMP/square.svg"
bleed 230 > "$TMP/rounded.svg"

ICONSET="$TMP/macx.iconset"; mkdir -p "$ICONSET"
for s in 16 32 128 256 512; do
  render "$SRC" "$s" "$ICONSET/icon_${s}x${s}.png"
  render "$SRC" $((s*2)) "$ICONSET/icon_${s}x${s}@2x.png"
done
iconutil -c icns "$ICONSET" -o deploy/macos/macx.icns

IOS=deploy/ios/Images.xcassets/AppIcon.appiconset
python3 - "$IOS/Contents.json" <<'PY' | while read -r f px; do render "$TMP/square.svg" "$px" "$IOS/$f"; done
import json,sys
seen=set()
for i in json.load(open(sys.argv[1]))["images"]:
    f=i.get("filename")
    if not f or f in seen: continue
    seen.add(f)
    print(f, int(float(i["size"].split("x")[0])*int(i["scale"][0])))
PY
render "$TMP/square.svg" 1024 deploy/ios/AppStoreIcon_1024x1024.png

for d in ldpi:36 mdpi:48 hdpi:72 xhdpi:96 xxhdpi:144 xxxhdpi:192; do
  render "$TMP/rounded.svg" "${d#*:}" "android/res/drawable-${d%:*}/icon.png"
done

for s in 16 24 32 48 64 128 256; do render "$TMP/rounded.svg" "$s" "$TMP/ico-$s.png"; done
magick "$TMP"/ico-{16,24,32,48,64,128,256}.png deploy/windows/WindowsQGC.ico
cp deploy/windows/WindowsQGC.ico resources/icons/qgroundcontrol.ico
