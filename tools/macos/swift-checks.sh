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
    "$root/macos/Sources/VibrationReading.swift" \
    "$root/macos/Sources/ParameterModel.swift" \
    "$root/macos/Sources/SensorHealth.swift" \
    "$root/macos/Sources/SafetySections.swift" \
    "$root/macos/Sources/Probeable.swift" \
    "$root/macos/Sources/MissionItemModel.swift" \
    "$root/macos/Sources/MissionCommandModel.swift" \
    "$root/macos/Sources/MissionItemKind.swift" \
    "$root/macos/Sources/FlyTelemetryModel.swift" \
    "$root/macos/Sources/VehicleMarkerModel.swift" \
    "$root/macos/Sources/ItemFactModel.swift" \
    "$root/macos/Sources/CameraChoiceModel.swift" \
    "$root/macos/Sources/AltitudeModeModel.swift" \
    "$root/macos/Sources/PlanSummaryModel.swift" \
    "$root/macos/Sources/MissionVehicleModel.swift" \
    "$root/macos/Sources/GeoTagModel.swift" \
    "$root/macos/Sources/TerrainProfileModel.swift" \
    "$root/macos/Sources/MapFraming.swift" \
    "$root/macos/Sources/FenceRallyModel.swift" \
    "$root/macos/Sources/TilePyramid.swift" \
    "$root/macos/Sources/VehicleComponentModel.swift" \
    "$root/macos/Sources/LogEntryModel.swift" \
    "$root/macos/Sources/FlightModePositions.swift" \
    "$root/macos/Sources/PageSelection.swift" \
    "$root/macos/Tests/main.swift"

"$out"
