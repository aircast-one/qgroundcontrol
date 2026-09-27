#!/usr/bin/env bash
# Typechecks every macOS head source that imports no UI framework, on a Linux Swift toolchain.
#
# SwiftUI, AppKit and MapKit exist only on Apple platforms, so the app cannot be built here. The
# stores and models can: 108 of the head's files import nothing but Foundation and the bridge's C
# modules. They are typechecked together, with linux-shims.swift standing in for Combine and
# linux-typecheck-stubs.swift for the handful of UI-side names they reach into. A file that starts
# importing a UI framework drops out of the set on its own; the count printed says how many ran.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
ui='^import (SwiftUI|AppKit|Cocoa|MapKit|AVFoundation|AVKit|CoreLocation|Metal|MetalKit|QuartzCore|CoreImage|IOKit|UniformTypeIdentifiers|WebKit)$'
mapfile -t sources < <(grep -LE "$ui" "$root"/macos/Sources/*.swift)

# Stores that import MapKit or CoreLocation only for a probe or a permission check, typechecked
# with those imports removed; the few names they use from them are stubbed.
stripped=(Mission.swift)
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
for name in "${stripped[@]}"; do
    grep -vE '^import (MapKit|CoreLocation)$' "$root/macos/Sources/$name" > "$work/$name"
    sources+=("$work/$name")
done

swiftc -typecheck -I "$root/src/Bridge" \
    "$root/tools/macos/linux-shims.swift" "$root/tools/macos/linux-typecheck-stubs.swift" "${sources[@]}"
echo "typechecked ${#sources[@]} macOS head sources: every one that imports no UI framework, and ${stripped[*]}"
