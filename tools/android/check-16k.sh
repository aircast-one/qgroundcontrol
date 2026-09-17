#!/bin/bash
set -euo pipefail
bundle=$1
ndk=${ANDROID_NDK_ROOT:-$HOME/Library/Android/sdk/ndk/27.2.12479018}
readelf=$(ls "$ndk"/toolchains/llvm/prebuilt/*/bin/llvm-readelf | head -1)
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
unzip -q "$bundle" 'base/lib/arm64-v8a/*.so' -d "$dir"
bad=0
for so in "$dir"/base/lib/arm64-v8a/*.so; do
    align=$("$readelf" -l "$so" | awk '/LOAD/ {print $NF}' | sort -u | tr '\n' ' ')
    case "$align" in *0x1000*|*0x2000*) echo "4K  $(basename "$so") $align"; bad=$((bad+1));; esac
done
total=$(ls "$dir"/base/lib/arm64-v8a/*.so | wc -l | tr -d ' ')
echo "$((total-bad))/$total arm64 libraries are 16 KB aligned"
[ "$bad" -eq 0 ]
