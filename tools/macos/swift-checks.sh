#!/bin/zsh
# Pure-logic checks for the Swift that needs no app, bridge or AppKit.
#
# Deliberately not a CMake target: a Swift executable target that shares sources with
# QGCNativeUI makes swiftc collide on swiftmodule temp paths
# (cannotResolveTempPath), and compiling three files directly is cheaper than
# working around it.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="${TMPDIR:-/tmp}/qgc-swift-checks"

swiftc -Onone -o "$out" \
    "$root/macos/Sources/Fact.swift" \
    "$root/macos/Sources/SettingsPages.swift" \
    "$root/macos/Sources/LinkConfigModel.swift" \
    "$root/macos/Tests/main.swift"

"$out"
