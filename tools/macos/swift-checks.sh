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

# The compile list below is written by hand, so a new model file is not checked until someone
# remembers to add it - and nothing says so, which is the same silence as a test that never runs.
# Every *Model*.swift must appear in it; anything else is a deliberate inclusion.
missing=()
for model in "$root"/macos/Sources/*Model*.swift; do
    grep -q "/${model:t}\"" "$0" || missing+=("${model:t}")
done
if (( ${#missing[@]} )); then
    print -u2 "swift-checks does not compile: ${missing[*]}"
    print -u2 "add them to the list in $0, or they go unchecked in silence"
    exit 1
fi

# systems.0 is whichever vehicle connected first. MAVLinkInspectorController::setMessageInterval
# acts on _activeSystem, which setActiveSystem(int) can point anywhere, so with two vehicles a
# head reading systems.0 shows one aircraft's messages while a rate change lands on another's.
# The core guards its own DEPS the same way; this guards the head's paths.
if grep -rn "mavlinkInspector\.systems\.0" "$root"/macos/Sources/*.swift; then
    print -u2 "the inspector must follow activeSystem, not systems.0 - see the lines above"
    exit 1
fi

# A probe action argument called "id" is unreachable: the request is /native/probe?id=<probe>,
# so the server reads that one and the action gets the probe's name where it wanted a number.
# It fails with a plausible error rather than obviously, which is how the notices dismiss action
# shipped never once having run.
if grep -rn 'args\["id"\]' "$root"/macos/Sources/*.swift; then
    print -u2 "a probe action argument named id collides with the probe selector - rename it"
    exit 1
fi

# textFieldFacts, comboboxFacts and the geoTag property names are interpolated into bridge
# paths and the core never names any of them, so nothing else checks their spelling.
"$root/tools/macos/interpolated-names.py" || exit 1

# SetupPage.bespoke has to name exactly what the content switch draws.
"$root/tools/macos/head-lists.py" || exit 1

swiftc -Onone -o "$out" \
    "$root/macos/Sources/DetectionModel.swift" \
    "$root/macos/Sources/SetupCatalogueModel.swift" \
    "$root/macos/Sources/SettingsPages.swift" \
    "$root/macos/Sources/AboutModel.swift" \
    "$root/macos/Sources/LinkConfigModel.swift" \
    "$root/macos/Sources/VibrationReading.swift" \
    "$root/macos/Sources/ParameterModel.swift" \
    "$root/macos/Sources/SensorHealth.swift" \
    "$root/macos/Sources/CalibrationModel.swift" \
    "$root/macos/Sources/RadioModel.swift" \
    "$root/macos/Sources/VideoModel.swift" \
    "$root/macos/Sources/VideoSourceModel.swift" \
    "$root/macos/Sources/CameraControlModel.swift" \
    "$root/macos/Sources/Probeable.swift" \
    "$root/macos/Sources/MissionItemModel.swift" \
    "$root/macos/Sources/MissionCommandModel.swift" \
    "$root/macos/Sources/MissionItemKind.swift" \
    "$root/macos/Sources/FlyStateModel.swift" \
    "$root/macos/Sources/FlyTelemetryModel.swift" \
    "$root/macos/Sources/FlyDetailModel.swift" \
    "$root/macos/Sources/VehicleLinksModel.swift" \
    "$root/macos/Sources/FlightModeModel.swift" \
    "$root/macos/Sources/VehicleTrackModel.swift" \
    "$root/macos/Sources/VehicleMarkerModel.swift" \
    "$root/macos/Sources/ItemFactModel.swift" \
    "$root/macos/Sources/CameraChoiceModel.swift" \
    "$root/macos/Sources/AltitudeModeModel.swift" \
    "$root/macos/Sources/PlanSummaryModel.swift" \
    "$root/macos/Sources/WriteReportModel.swift" \
    "$root/macos/Sources/ItemSpeedModel.swift" \
    "$root/macos/Sources/LaunchPositionModel.swift" \
    "$root/macos/Sources/MeasureModel.swift" \
    "$root/macos/Sources/SetupPageModel.swift" \
    "$root/macos/Sources/MapClickModel.swift" \
    "$root/macos/Sources/MapCentreModel.swift" \
    "$root/macos/Sources/FlyOverlayModel.swift" \
    "$root/macos/Sources/MapFollowModel.swift" \
    "$root/macos/Sources/MapScaleModel.swift" \
    "$root/macos/Sources/TerrainDownloadModel.swift" \
    "$root/macos/Sources/PolygonEditModel.swift" \
    "$root/macos/Sources/MotorTestModel.swift" \
    "$root/macos/Sources/RemoteSupportModel.swift" \
    "$root/macos/Sources/VehicleSetupTextModel.swift" \
    "$root/macos/Sources/FenceSupportModel.swift" \
    "$root/macos/Sources/PlanActionsModel.swift" \
    "$root/macos/Sources/PatternGeometryModel.swift" \
    "$root/macos/Sources/SurveyStatsModel.swift" \
    "$root/macos/Sources/MissionVehicleModel.swift" \
    "$root/macos/Sources/GeoTagModel.swift" \
    "$root/macos/Sources/MavlinkInspectorModel.swift" \
    "$root/macos/Sources/BatteryReadingModel.swift" \
    "$root/macos/Sources/FrameSetupModel.swift" \
    "$root/macos/Sources/VehicleMessageModel.swift" \
    "$root/macos/Sources/PreflightModel.swift" \
    "$root/macos/Sources/VehicleWarningModel.swift" \
    "$root/macos/Sources/InstrumentModel.swift" \
    "$root/macos/Sources/GuidedActionsModel.swift" \
    "$root/macos/Sources/GuidedValueModel.swift" \
    "$root/macos/Sources/TerrainProfileModel.swift" \
    "$root/macos/Sources/MapFraming.swift" \
    "$root/macos/Sources/FenceRallyModel.swift" \
    "$root/macos/Sources/TilePyramid.swift" \
    "$root/macos/Sources/VehicleComponentModel.swift" \
    "$root/macos/Sources/LogEntryModel.swift" \
    "$root/macos/Sources/HostNoticeModel.swift" \
    "$root/macos/Sources/MavlinkConsoleModel.swift" \
    "$root/macos/Sources/ModeSlotsModel.swift" \
    "$root/macos/Sources/MissionSummaryModel.swift" \
    "$root/macos/Sources/BridgeWatchModel.swift" \
    "$root/macos/Sources/VideoFrameModel.swift" \
    "$root/macos/Sources/FlightModePositions.swift" \
    "$root/macos/Sources/PageSelection.swift" \
    "$root/macos/Tests/main.swift"

QGC_ROOT="$root" "$out"
