import json
import os
import re
import subprocess
import sys

CONTRACT = os.environ.get(
    "VIEW_SHAPES",
    os.path.expanduser("~/Code/aircast/qgroundcontrol/test/Bridge/fixtures/view-shapes.json"),
)
BASELINE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "unread-baseline.txt")
READS = re.compile(r'\bopt(?:Text|Boolean|Int|Long|Double|JSONObject|JSONArray|String)\(\s*"([A-Za-z][A-Za-z0-9]*)"')
NAMES_VIEW = re.compile(r'"(view\.[A-Za-z][A-Za-z0-9]*)')
NESTED = re.compile(r'\.optJSON(?:Object|Array)\(\s*"([A-Za-z][A-Za-z0-9]*)"')
BARE_BOOL = re.compile(r'\.optBoolean\(\s*"([A-Za-z][A-Za-z0-9]*)"\s*\)')
HELPER_DEF = re.compile(r'fun (?:<[^>]+>\s*)?JSON(?:Object|Array)\.([a-zA-Z][A-Za-z0-9]*)\(\s*[a-zA-Z]+: String')
TAKES_KEY = re.compile(
    r'fun ([a-zA-Z][A-Za-z0-9]*)\(\s*[a-zA-Z]+: JSON(?:Object|Array)\??\s*,\s*key: String'
)


def fixture_provenance():
    repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    rel = os.path.relpath(CONTRACT, repo)
    dirty = subprocess.run(
        ["git", "-C", repo, "status", "--porcelain", "--", rel],
        capture_output=True, text=True,
    ).stdout.strip()
    return rel if dirty else ""


def served(node, into):
    if isinstance(node, dict):
        for key, value in node.items():
            if "." not in key:
                into.add(key)
            served(value, into)
    elif isinstance(node, list):
        for item in node:
            served(item, into)
    return into


def groups(node, root, into):
    if isinstance(node, dict):
        names = {k for k in node if "." not in k}
        if len(names) > 1:
            into.append((root, names))
        for value in node.values():
            groups(value, root, into)
    elif isinstance(node, list):
        for item in node:
            groups(item, root, into)
    return into


def head_views(root):
    found = set()
    for base, _, names in os.walk(root):
        if "/build/" in base or "/test/" in base:
            continue
        for name in names:
            if name.endswith(".kt"):
                found.update(NAMES_VIEW.findall(open(os.path.join(base, name)).read()))
    return found


def not_recorded(android):
    test = os.path.join(os.path.dirname(android), "test", "Bridge", "QGCCoreCTest.cc")
    table = re.search(r"kNotRecorded = \{(.*?)\n    \};", open(test).read(), re.S)
    return set(re.findall(r'\{ QStringLiteral\("(view\.[A-Za-z0-9]+)"\)', table.group(1))) if table else set()


def recorded_null(node, into):
    if isinstance(node, dict):
        for key, value in node.items():
            if value == "null":
                into.add(key)
            else:
                recorded_null(value, into)
    elif isinstance(node, list):
        for item in node:
            recorded_null(item, into)
    return into


def opened_as_container(root):
    found = set()
    for base, _, names in os.walk(root):
        if "/build/" in base or "/test/" in base:
            continue
        for name in names:
            if name.endswith(".kt"):
                found |= set(NESTED.findall(open(os.path.join(base, name)).read()))
    return found


def recorded_empty(node, path, into):
    if isinstance(node, dict):
        for key, value in node.items():
            if value == ["empty"]:
                into.append(f"{path}/{key}")
            else:
                recorded_empty(value, f"{path}/{key}", into)
    elif isinstance(node, list):
        for item in node:
            recorded_empty(item, path, into)
    return into


def nullable_bools(node, path, into):
    if isinstance(node, dict):
        for key, value in node.items():
            if isinstance(value, str) and "null" in value.split("|") and "bool" in value.split("|"):
                into.setdefault(key, set()).add(path or key)
            else:
                nullable_bools(value, path or key, into)
    elif isinstance(node, list):
        for item in node:
            nullable_bools(item, path, into)
    return into


def flattened(root, nullable):
    bare = re.compile(r'\.optBoolean\(\s*"([A-Za-z][A-Za-z0-9]*)"\s*\)')
    hits = {}
    for base, _, names in os.walk(root):
        if "/build/" in base or "/test/" in base:
            continue
        for name in names:
            if not name.endswith(".kt"):
                continue
            body = open(os.path.join(base, name)).read()
            for key in bare.findall(body):
                if key in nullable and key not in ACCEPTED_FLAT and f'isNull("{key}")' not in body:
                    hits.setdefault(key, set()).add(name)
    return hits


ACCEPTED_FLAT = {
    "enabled": "only view.obstacle.enabled is an Option, and neither obstacle file reads it; every bare read is another object's plain bool",
    "supported": "only view.obstacle.supported is an Option, and CameraControl.kt reads the camera tracking object's plain bool",
    "orbiting": "OrbitMarker.kt hides the circle on null like QGC, whose 3 s _orbitTelemetryTimer clears orbitActive whether or not contact is lost",
    "ready": "PlanFileRules and VehicleSync read the plan's own ready, not view.setup.ready -"
        " SetupView.kt is the one that reads that, and it holds it as Boolean?",
    "stale": "view.detections.stale and an adsbTraffic contact's stale are both plain bool;"
        " only view.obstacle.stale is an Option, and ObstacleDistance.kt names its choice",
}

def head_keys(root):
    sources = {}
    for base, _, names in os.walk(root):
        if "/build/" in base or "/test/" in base:
            continue
        for name in names:
            if name.endswith(".kt"):
                sources[os.path.join(base, name)] = open(os.path.join(base, name)).read()

    helpers = {h for src in sources.values() for h in HELPER_DEF.findall(src)}
    through = re.compile(
        r'\.(?:' + "|".join(sorted(helpers)) + r')\(\s*"([A-Za-z][A-Za-z0-9]*)"'
    ) if helpers else None

    takers = {t for src in sources.values() for t in TAKES_KEY.findall(src)}
    passed = re.compile(
        r'\b(?:' + "|".join(sorted(takers)) + r')\(\s*[^,()]+,\s*"([A-Za-z][A-Za-z0-9]*)"'
    ) if takers else None

    found = {}
    for path, src in sources.items():
        keys = set(READS.findall(src))
        if through:
            keys |= set(through.findall(src))
        if passed:
            keys |= set(passed.findall(src))
        for key in keys:
            found.setdefault(key, set()).add(os.path.basename(path))
    return found


ACCEPTED_ON = {
    ("view.gcsPosition", "distanceToVehicle"): "the head reads distanceToVehicleText, the core's own formatting",
    ("view.gcsPosition", "distanceToVehicleMeters"): "raw half of distanceToVehicleText",
    ("view.gcsPosition", "distanceToVehicleUnits"): "unit half of distanceToVehicleText",
    ("view.missionItems", "altitudeFrame"): "the head reads altitudeFrameText",
    ("view.plan", "defaultValue"): "the head reads defaultText and only ever SHOWS a default, never writes one",
    ("view.settings", "defaultValue"): "see view.plan",
    ("view.plan", "valueMeters"): "the geometry half of value. MEASURED: no consumer on this head feeds a "
        "control value into geometry - settings round-trip through Qgc.set, which is cooked and correct",
    ("view.settings", "valueMeters"): "see view.plan",
    ("view.vehicleLinks", "watching"): "the reason sentence the core serves alongside says the same thing",
    ("view.vehicleLinks", "autoDisconnect"): "contact loss reaches the operator through view.flyState",
    ("view.plan", "speedUnits"): "the unit the two speed defaults share. Each control carries its "
        "own units and PlanDefaultsDialog renders them through FactRow, so this is the core "
        "telling a head that wants ONE label for both - this one does not",
    ("view.frame", "vehicleTypeText"): "view.frame is read by MotorsScreen for motorCount and the safety gate; "
        "naming the airframe belongs to a setup summary, not the motor test",
    ("view.aircastLink", "quality"): "the head reads qualityText; the Aircast link indicator has no QGC counterpart",
    ("view.appLog", "empty"): "AppLogPage.kt prints the same No log entries line as AppLogging.qml",
    ("view.appLog", "headers"): "AppLogging.qml draws no column headers",
    ("view.battery", "secondaryText"): "the macOS head's pack subtitle; the Android battery sheet shows indicatorLines and rows like BatteryIndicator.qml",
    ("view.camera", "canStopPhoto"): "the gate actions.rs checks for stopTakePhoto; the shutter's stop state reaches the head through panel, the PhotoVideoControl.qml port",
    ("view.camera", "modeKnown"): "hasModes and panel carry PhotoVideoControl.qml's mode toggle rule; modeKnown is the core's own input to it",
    ("view.control", "defaultValue"): "a Fact's default; heads show it through defaultText or Qgc.kt defaultValueString and never write it",
    ("view.control", "rebootRequired"): "the core posts QGC's reboot prompt itself after an accepted write (factwrite hook, 4093b4abe), so no head acts on the flag",
    ("view.control", "valueMeters"): "the geometry half of value; FactRow round-trips the cooked value through Qgc.set",
    ("view.firmware", "firmwareType"): "a part of summary, the one line SetupView shows",
    ("view.firmware", "version"): "a part of summary, the one line SetupView shows",
    ("view.firmware", "versionType"): "a part of summary, the one line SetupView shows",
    ("view.firmwareUpgrade", "board"): "FirmwareUpgradeController shows the board only as status log lines (Connected to bootloader, Board ID, Flash size), which firmwareflash.rs posts to messages and FirmwareScreen lists",
    ("view.firstRun", "defaultValue"): "a Fact's default; heads show it through defaultText or Qgc.kt defaultValueString and never write it",
    ("view.firstRun", "promptId"): "the core's key for the dismissed-prompt store; the head acts on show",
    ("view.firstRun", "rebootRequired"): "the core posts QGC's reboot prompt itself after an accepted write (factwrite hook, 4093b4abe), so no head acts on the flag",
    ("view.firstRun", "valueMeters"): "the geometry half of value; FactRow round-trips the cooked value through Qgc.set",
    ("view.flightModes", "section"): "the core's grouping it sorts by and derives needsConfirm from; FlightModeIndicator lists modes flat",
    ("view.flyMissionItems", "altitudeFrame"): "the head reads altitudeFrameText, as for view.missionItems",
    ("view.frame", "apmFirmware"): "MotorsScreen reads motorCount and the safety gate; the firmware branch is taken in the core",
    ("view.gpsRtk", "accuracyUnit"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "altitudeUnit"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "attemptsRemaining"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "configureFailures"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "driver"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "durationUnit"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "positionKnown"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "relative"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "rtcmBytes"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "rtcmFreshness"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "satelliteFreshness"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "satellitesUsed"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "surveyFreshness"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "targetAccuracy"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "targetDuration"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.gpsRtk", "withinAccuracyLimit"): "GPSIndicatorPage.qml's RTK GPS Status shows only active, numSatellites, currentDuration, currentAccuracy and valid, which RtkIndicator.kt reads; the rest is core survey/driver diagnostics QGC never shows",
    ("view.guidedActions", "roiActive"): "folded by the core into offers and roi, which GuidedActions.kt and RoiMarker.kt read",
    ("view.guidedActions", "roiSupported"): "folded by the core into offers, which GuidedActions.kt reads",
    ("view.hostNotices", "acknowledgedThrough"): "HostNotices.kt reads the core's split of the queue into banners, dialogs and unseen; these are the raw queue and its acknowledgement bookkeeping",
    ("view.hostNotices", "latestId"): "HostNotices.kt reads the core's split of the queue into banners, dialogs and unseen; these are the raw queue and its acknowledgement bookkeeping",
    ("view.hostNotices", "notices"): "HostNotices.kt reads the core's split of the queue into banners, dialogs and unseen; these are the raw queue and its acknowledgement bookkeeping",
    ("view.itemFacts", "customName"): "CameraCalcSection reads custom (isCustomCamera) and brands, which already lists Custom Camera like CameraCalc's cameraBrandList",
    ("view.itemFacts", "defaultValue"): "a Fact's default; heads show it through defaultText or Qgc.kt defaultValueString and never write it",
    ("view.itemFacts", "rebootRequired"): "the core posts QGC's reboot prompt itself after an accepted write (factwrite hook, 4093b4abe), so no head acts on the flag",
    ("view.itemFacts", "valueMeters"): "the geometry half of value; FactRow round-trips the cooked value through Qgc.set",
    ("view.mapTypes", "provider"): "the core filters types by provider (maptypes.rs); the head lists types and current",
    ("view.modeSlots", "channelPwm"): "APMFlightModesComponent.qml shows fixed PWM band labels and highlights the live slot, never the raw channel PWM",
    ("view.offlineMaps", "maxTilesForDownload"): "estimate.tooMany carries the limit; OfflineMapEditor shows only Too many tiles",
    ("view.offlineMaps", "tileCount"): "the head reads tileCountText, the same count formatted",
    ("view.operatorControl", "systemManager"): "GCSControlIndicator.qml declares gcsControlStatusFlags_SystemManager and never shows it",
    ("view.operatorControl", "takeoverTimeoutMs"): "QGC's revert popup shows only the seconds left, which the core serves as takeoverRevertMs",
    ("view.packetRadio", "adapter"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "antennaCount"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "antennaRssi"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "antennaScore"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "channelWidth"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "keyPath"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "keySource"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "linkScoreMax"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "linkScoreMin"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "packetLossUnit"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "pollIntervalMs"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "polling"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "readingAgeMs"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "rejectedKeyPath"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "retryArmed"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "retryIntervalMs"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "rssiUnit"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "snrUnit"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "startError"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "starting"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "unsupportedAdapters"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "videoHost"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "videoOverridden"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.packetRadio", "videoPort"): "packet radio (wfb-ng) is an Aircast addition with no QGC screen to match; PacketRadioSection.kt shows the rows its design asks for",
    ("view.plan", "apmFirmware"): "planningFor flag plan.rs branches on; PlanVehicleRows shows the firmware and vehicle text like PlanViewToolBar",
    ("view.plan", "descent"): "read through PLAN_DEFAULT_KEYS in PlanDefaults.kt",
    ("view.plan", "download"): "read by the allowed(\"download\") helper in PlanFileRules",
    ("view.plan", "rebootRequired"): "the core posts QGC's reboot prompt itself after an accepted write (factwrite hook, 4093b4abe), so no head acts on the flag",
    ("view.planTransform", "coordinateSystems"): "ItemEditor's CoordinateSystem lists the same four systems and offers Vehicle Position only while hasVehicle(), as the served list does",
    ("view.planTransform", "hasHome"): "the head tests home for null, the same fact",
    ("view.planTransform", "verticalMetresPerUnit"): "read by transformUnit(view, \"vertical\") through a ${axis}MetresPerUnit string template",
    ("view.planTransform", "verticalUnit"): "read by transformUnit(view, \"vertical\") through a ${axis}Unit string template",
    ("view.px4Tuning", "defaultValue"): "a Fact's default; heads show it through defaultText or Qgc.kt defaultValueString and never write it",
    ("view.px4Tuning", "rebootRequired"): "the core posts QGC's reboot prompt itself after an accepted write (factwrite hook, 4093b4abe), so no head acts on the flag",
    ("view.px4Tuning", "valueMeters"): "the geometry half of value; FactRow round-trips the cooked value through Qgc.set",
    ("view.sensorSettings", "defaultValue"): "a Fact's default; heads show it through defaultText or Qgc.kt defaultValueString and never write it",
    ("view.sensorSettings", "rebootRequired"): "the core posts QGC's reboot prompt itself after an accepted write (factwrite hook, 4093b4abe), so no head acts on the flag",
    ("view.sensorSettings", "valueMeters"): "the geometry half of value; FactRow round-trips the cooked value through Qgc.set",
    ("view.settings", "rebootRequired"): "the core posts QGC's reboot prompt itself after an accepted write (factwrite hook, 4093b4abe), so no head acts on the flag",
    ("view.terrainDownload", "started"): "total > 0; TerrainProgress shows loaded of total",
    ("view.terrainProfile", "collidingItems"): "read by collidingItems(view) in TerrainProfile.kt, whose key is a default argument the read pattern cannot see",
    ("view.vehicles", "missionFlightMode"): "an input to vehicles.rs action gating; the fleet panel shows no mission mode, like MultiVehicleList",
    ("view.vehicles", "pauseSupported"): "an input to vehicles.rs mvPause gating, which the fleet actions already obey",
    ("view.vehicles", "selectedCount"): "the fleet panel heading lists the selected ids it holds, like MultiVehicleList; the count feeds the core's own gates",
    ("view.video", "nativePipeline"): "native GStreamer host state for the video surface driver; QGC shows no pipeline",
    ("view.video", "nativeRecording"): "native recording file for the video surface driver; recording state reaches the operator through view.camera",
}

ONLY_IN = {
    "enumStrings": {"Qgc.kt"},
    "enumIndex": {"Qgc.kt"},
    "bitmaskStrings": {"Qgc.kt"},
    "bitmaskValues": {"Qgc.kt"},
    "typeIsBool": {"Qgc.kt"},
    "typeIsInteger": {"Qgc.kt"},
    "defaultValueString": {"Qgc.kt"},
    "maxString": {"Qgc.kt"},
    "minString": {"Qgc.kt"},
    "unknownEnumLabel": {"Qgc.kt"},
    "defaultValueAvailable": {"Qgc.kt", "ParameterLinks.kt"},
    "ports": {"FirmwareScreen.kt"},
    "bootloader": {"FirmwareScreen.kt"},
    "statusId": {"LogDownloadScreen.kt"},
    "hold": {"WaypointSpeed.kt"},
    "valueEqualsDefault": {"Qgc.kt"},
    "message": {"VehicleMessages.kt"},
    "values": {"InstrumentDisplay.kt"},
    "colours": {"InstrumentDisplay.kt"},
    "opacities": {"InstrumentDisplay.kt"},
    "icons": {"InstrumentDisplay.kt"},
    "serial": {"FlightModes.kt"},
    "size": {"Px4LogTransfer.kt"},
}

ACCEPTED = {
    "values": "InstrumentDisplay.kt reads back the value display style JSON its own displayJson writes to settings, not a view",
    "colours": "InstrumentDisplay.kt reads back the value display style JSON its own displayJson writes to settings, not a view",
    "opacities": "InstrumentDisplay.kt reads back the value display style JSON its own displayJson writes to settings, not a view",
    "icons": "InstrumentDisplay.kt reads back the value display style JSON its own displayJson writes to settings, not a view",
    "serial": "view.flightModes modeAck is null until a mode change is acknowledged, and the rig records none; flightmodes.rs serves serial with accepted and wording",
    "size": "view.mavlinkLog files lists saved MAVLink logs and the rig records none; mavlinklog.rs serves size per file",
    "timeoutMs": "view.operatorControl incomingRequest is null until another GCS asks for control, and the rig has none; operatorcontrol.rs incoming_json serves it",
    "remainingMs": "view.operatorControl incomingRequest is null until another GCS asks for control, and the rig has none; operatorcontrol.rs incoming_json serves it",
    "uniqueCount": "view.offlineMaps sets lists saved tile sets and the rig records none; offlinemaps.rs set_json serves uniqueCount per set",
    "ports": "view.firmwarePorts is in QGCCoreCTest kNotRecorded because it lists the serial ports of the recording machine; firmwareflash.rs ports_view serves it",
    "bootloader": "view.firmwarePorts is in QGCCoreCTest kNotRecorded because it lists the serial ports of the recording machine; firmwareflash.rs ports serves bootloader per port",
    "statusId": "view.logs lists the vehicle's onboard logs and the rig records before any log list arrives, so entries records empty; logs.rs logs_view serves statusId per entry",
    "hold": "view.itemFacts is recorded under the Qt plan, and hold is served only by the core plan (coreplan.rs hold_field); the Qt plan shows the waypoint's Hold param among its fields",
    "compass": "view.calibration compassResults lists APM compasses after an onboard compass calibration, and the rig records with no compass calibrated, so the list records empty; calibration.rs compass_results serves it",
    "green": "view.calibration compassResults lists APM compasses after an onboard compass calibration, and the rig records with no compass calibrated, so the list records empty; calibration.rs compass_results serves it",
    "yellow": "view.calibration compassResults lists APM compasses after an onboard compass calibration, and the rig records with no compass calibrated, so the list records empty; calibration.rs compass_results serves it",
    "boardType": "view.firmwarePorts lists USB serial ports and the desktop rig records with none attached, so the list records empty; firmwareflash.rs recognized_board serves it on Android",
    "layerHeight": "view.surveyStats structure rows exist only for a structure scan, and the rig's plan has none, so structure records null; survey.rs layer_rows serves them from StructureScanComplexItem's topFlightAlt/bottomFlightAlt",
    "top": "view.surveyStats structure rows exist only for a structure scan, and the rig's plan has none, so structure records null; survey.rs layer_rows serves them from StructureScanComplexItem's topFlightAlt/bottomFlightAlt",
    "bottom": "view.surveyStats structure rows exist only for a structure scan, and the rig's plan has none, so structure records null; survey.rs layer_rows serves them from StructureScanComplexItem's topFlightAlt/bottomFlightAlt",
    "east": "view.viewer3d bounds and buildings exist only once the 3D view is on with a readable OpenStreetMap file, and the rig has neither, so they record empty; viewer3d.rs viewer3d_view serves them from QGC's OsmParser rules",
    "west": "view.viewer3d bounds and buildings exist only once the 3D view is on with a readable OpenStreetMap file, and the rig has neither, so they record empty; viewer3d.rs viewer3d_view serves them from QGC's OsmParser rules",
    "north": "view.viewer3d bounds and buildings exist only once the 3D view is on with a readable OpenStreetMap file, and the rig has neither, so they record empty; viewer3d.rs viewer3d_view serves them from QGC's OsmParser rules",
    "south": "view.viewer3d bounds and buildings exist only once the 3D view is on with a readable OpenStreetMap file, and the rig has neither, so they record empty; viewer3d.rs viewer3d_view serves them from QGC's OsmParser rules",
    "outer": "view.viewer3d bounds and buildings exist only once the 3D view is on with a readable OpenStreetMap file, and the rig has neither, so they record empty; viewer3d.rs viewer3d_view serves them from QGC's OsmParser rules",
    "inner": "view.viewer3d bounds and buildings exist only once the 3D view is on with a readable OpenStreetMap file, and the rig has neither, so they record empty; viewer3d.rs viewer3d_view serves them from QGC's OsmParser rules",
    "inverted": "view.settings controls[].inverted marks a switch shown flipped, like TelemetrySettings.qml Controlled by Vehicle (checked: !apmStartMavlinkStreams); only an ArduPilot vehicle or none shows that row and the rig records none (settings.rs INVERTED). Read in ParameterForm.kt",
    "indent": "view.setup rows from a VehicleConfig control marked indent (ArduPilot help labels); the rig's PX4 mock pages carry none. Read in ParameterForm.kt",
    "firstEntryIsAll": "view.setup on the ArduPilot Flight Safety page (ARMING_CHECK); the rig records a PX4 mock, so no such bitmask is served. Read in ParameterForm.kt",
    "wording": "view.flightModes.modeAck is the core hub's last DO_SET_MODE ack; the Qt rig has no hub vehicle, so it is null there. Read in FlightModes.kt",
    "ok": "the invoke envelope, not a view field",
    "componentId": "a parameterFile.review row (an invoke result, not a view field); paramfile.rs serves it. Read in ParameterTools.kt",
    "noVehicleValue": "a parameterFile.review row (an invoke result, not a view field); paramfile.rs serves it. Read in ParameterTools.kt",
    "onScreen": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "clickAndDrag": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "controlLabel": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "controlOffered": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "deviceId": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "haveControl": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "managerCompid": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "pitchText": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "retractOffered": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "yawLockLabel": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "yawLocked": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "yawLockOffered": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "yawText": "view.gimbalIndicator with a gimbal attached; the rig has none, so the recording holds only shown:false. Read in GimbalIndicator.kt",
    "result": "the invoke envelope, not a view field",
    "otherVehicle": "parameterFile.review's answer, an invoke result rather than a view",
    "multipleComponents": "parameterFile.review's answer, an invoke result rather than a view",
    "fileValue": "a parameterFile.review row, an invoke result rather than a view",
    "vehicleValue": "a parameterFile.review row, an invoke result rather than a view",
    "cannotSend": "a parameterFile.review row, an invoke result rather than a view",
    "friendlyName": "missionCommandTree.getCommandsForCategory's MissionCommandUIInfo, an invoke result rather than a view",
    "defaultRadius": "a field of view.mapClick.loiter, which the contract records as NULL because "
        "the rig never has a fixed wing loitering after a goto. Read in MapClickMenu.kt",
    "simulated": "served per contact at adsb.rs:386 and invisible here because"
        "view.adsbTraffic.contacts records EMPTY - the same blind spot this check already"
        "prints above. Read in TrafficView.kt since 457dbdda7",
    "elements": "the bridge's list envelope",
    "shortDescription": "the vehicle's own fact group, read as a raw object because"
        "view.instrumentGroups enumerates the vehicle's CHILD groups and skips its own. That is"
        "where the four readings the flight screen starts with live, so without reading it the"
        "picker cannot draw them as chosen or let them be turned off. The macOS head builds the"
        "same group the same way",
    "defaultValueString": "Fact metadata read by path",
    "defaultValueAvailable": "Fact metadata read by path, for the parameter editor's Modified filter",
    "valueEqualsDefault": "Fact metadata read by path, for the parameter editor's Modified filter",
    "maxString": "Fact metadata read by path",
    "minString": "Fact metadata read by path",
    "unknownEnumLabel": "Fact metadata read by path",
    "alert": "per-contact and invisible here because view.adsbTraffic.contacts records EMPTY, "
        "the blind spot this check prints below. Read in TrafficView.kt",
    "bearingDegrees": "per-contact, see alert",
    "relativeAltitude": "per-contact, see alert",
    "enumStrings": "Fact metadata read by path",
    "enumIndex": "Fact metadata read by path",
    "bitmaskStrings": "Fact metadata read by path",
    "bitmaskValues": "Fact metadata read by path",
    "typeIsBool": "Fact metadata read by path",
    "typeIsInteger": "Fact metadata read by path",
    "typeIsString": "Fact metadata read by path",
    "minIsDefaultForType": "Fact metadata read by path",
    "maxIsDefaultForType": "Fact metadata read by path",
    "qgcRebootRequired": "Fact metadata read by path",
    "width": "the video surface, not a core view",
    "height": "the video surface, not a core view",
    "received": "the log download screen reads its own progress shape",
    "sizeText": "the log download screen reads its own progress shape",
    "timeState": "the log download screen reads its own progress shape",
    "close": "view.obstacle recorded without the nearest object present",
    "sectorText": "view.obstacle recorded without the nearest object present",
    "w": "view.detections recorded with no box, so boxes[] pins no element shape",
    "h": "view.detections recorded with no box, so boxes[] pins no element shape",
    "confidence": "view.detections recorded with no box, so boxes[] pins no element shape",
    "callsign": "view.adsbTraffic recorded with contacts[] empty, so no contact shape is pinned;"
        " adsb.rs serves it and a probe read SWR000 from the live head",
    "icaoAddress": "view.adsbTraffic recorded with contacts[] empty; adsb.rs serves it",
    "altitudeType": "view.adsbTraffic recorded with contacts[] empty; adsb.rs serves it",
    "headingDegrees": "view.adsbTraffic recorded with contacts[] empty; adsb.rs serves it",
    "canDelete": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "rowText": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "pitchDegrees": "view.gimbalIndicator records shown:false because the recording has no gimbal; gimbalindicator.rs indicator serves it",
    "capturing": "view.camera records panel.video and panel.photo null because the recording has no camera; video.rs photo_video_panel serves it",
    "clock": "view.camera records panel.video and panel.photo null because the recording has no camera; video.rs photo_video_panel serves it",
    "idle": "view.camera records panel.video and panel.photo null because the recording has no camera; video.rs photo_video_panel serves it",
    "press": "view.camera records panel.video and panel.photo null because the recording has no camera; video.rs photo_video_panel serves it",
    "fileName": "geoTag records imageModel empty because the recording tags no images; geotagcontroller.rs object serves its rows",
    "imageModel": "geoTag records imageModel empty because the recording tags no images; geotagcontroller.rs object serves its rows",
    "statusString": "geoTag records imageModel empty because the recording tags no images; geotagcontroller.rs object serves its rows",
    "subtitle": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "complete": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "defaultSet": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "downloadStatus": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "downloadedText": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "errorCount": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "errorCountText": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "mapTypeStr": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "totalText": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "uniqueText": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "zoomText": "view.offlineMaps records sets[] empty because the recording has no tile cache open; offlinemaps.rs set_json serves it",
    "bit": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "columns": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "configs": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "groupsVisible": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "showAs": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "showUi": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "subgroups": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "actuators": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "allMotors": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "function": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "hadFailure": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "isMotor": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "testing": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "cells": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "fixed": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "hasUnsetRequiredFunctions": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "helpUrl": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "notes": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "channelFunction": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs channel_cells serves it",
    "disabled": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs channel_cells serves it",
    "counterClockwise": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs motor_geometry serves it",
    "highlighted": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "motorAssignment": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "multirotor": "view.actuatorOutputs is served from the core hub's actuator metadata, which the Qt recording has none of; actuators.rs outputs_json serves it",
    "position": "view.apmServos lists ArduPilot SERVOn outputs and the recording rig is PX4, so its list is empty; apmservos.rs servos_view serves it",
    "pwm": "view.apmServos lists ArduPilot SERVOn outputs and the recording rig is PX4, so its list is empty; apmservos.rs servos_view serves it",
    "angle": "view.apmFollow serves ArduPilot FOLL_* parameters and the recording rig is PX4, so it records only available=false; apmfollow.rs follow_view serves it",
    "pointIndex": "view.apmFollow serves ArduPilot FOLL_* parameters and the recording rig is PX4, so it records only available=false; apmfollow.rs follow_view serves it",
    "pointOptions": "view.apmFollow serves ArduPilot FOLL_* parameters and the recording rig is PX4, so it records only available=false; apmfollow.rs follow_view serves it",
    "positionIndex": "view.apmFollow serves ArduPilot FOLL_* parameters and the recording rig is PX4, so it records only available=false; apmfollow.rs follow_view serves it",
    "positionOptions": "view.apmFollow serves ArduPilot FOLL_* parameters and the recording rig is PX4, so it records only available=false; apmfollow.rs follow_view serves it",
    "rover": "view.apmFollow serves ArduPilot FOLL_* parameters and the recording rig is PX4, so it records only available=false; apmfollow.rs follow_view serves it",
    "showSettings": "view.apmFollow serves ArduPilot FOLL_* parameters and the recording rig is PX4, so it records only available=false; apmfollow.rs follow_view serves it",
    "waiting": "view.apmFollow serves ArduPilot FOLL_* parameters and the recording rig is PX4, so it records only available=false; apmfollow.rs follow_view serves it",
    "inUse": "view.signingKeys lists the core key store, which the desktop recording rig has no keys in, so its list is empty; signingkeys.rs signing_keys_view serves it",
    "activeOnVehicle": "view.signingKeys lists the core key store, which the desktop recording rig has no keys in, so its list is empty; signingkeys.rs signing_keys_view serves it",
    "completed": "joystick.calibration's action answer, not a view; joystickhost.rs calibration() serves it",
    "oneSidedVisible": "view.joystick's calibration block exists only while a stick is being calibrated, and the desktop rig has no gamepad; stickcal.rs json serves it",
    "buttons": "view.joystick's state (axes, buttons and their events) exists only with a gamepad attached, and the desktop rig has none; joystick.rs snapshot serves it",
    "event": "view.joystick's state (axes, buttons and their events) exists only with a gamepad attached, and the desktop rig has none; joystick.rs snapshot serves it",
    "baudIndex": "view.espBridge serves the ESP8266 bridge's component-240 parameters and the desktop rig has no bridge, so it records only available=false; espbridge.rs esp_bridge_view serves it",
    "bridge": "view.espBridge serves the ESP8266 bridge's component-240 parameters and the desktop rig has no bridge, so it records only available=false; espbridge.rs esp_bridge_view serves it",
    "channelPath": "view.espBridge serves the ESP8266 bridge's component-240 parameters and the desktop rig has no bridge, so it records only available=false; espbridge.rs esp_bridge_view serves it",
    "hostPort": "view.espBridge serves the ESP8266 bridge's component-240 parameters and the desktop rig has no bridge, so it records only available=false; espbridge.rs esp_bridge_view serves it",
    "modeIndex": "view.espBridge serves the ESP8266 bridge's component-240 parameters and the desktop rig has no bridge, so it records only available=false; espbridge.rs esp_bridge_view serves it",
    "modePath": "view.espBridge serves the ESP8266 bridge's component-240 parameters and the desktop rig has no bridge, so it records only available=false; espbridge.rs esp_bridge_view serves it",
    "password": "view.espBridge serves the ESP8266 bridge's component-240 parameters and the desktop rig has no bridge, so it records only available=false; espbridge.rs esp_bridge_view serves it",
    "passwordSta": "view.espBridge serves the ESP8266 bridge's component-240 parameters and the desktop rig has no bridge, so it records only available=false; espbridge.rs esp_bridge_view serves it",
    "qgc": "view.espBridge serves the ESP8266 bridge's component-240 parameters and the desktop rig has no bridge, so it records only available=false; espbridge.rs esp_bridge_view serves it",
    "rebootPrompt": "view.espBridge serves the ESP8266 bridge's component-240 parameters and the desktop rig has no bridge, so it records only available=false; espbridge.rs esp_bridge_view serves it",
    "ssid": "view.espBridge serves the ESP8266 bridge's component-240 parameters and the desktop rig has no bridge, so it records only available=false; espbridge.rs esp_bridge_view serves it",
    "ssidSta": "view.espBridge serves the ESP8266 bridge's component-240 parameters and the desktop rig has no bridge, so it records only available=false; espbridge.rs esp_bridge_view serves it",
    "autoDecPath": "view.sensorSettings is ArduPilot's AHRS/compass setup and the recording rig is PX4, so it records only available=false; sensorsettings.rs sensor_settings_view serves it",
    "manual": "view.sensorSettings is ArduPilot's AHRS/compass setup and the recording rig is PX4, so it records only available=false; sensorsettings.rs sensor_settings_view serves it",
    "orientation": "view.sensorSettings is ArduPilot's AHRS/compass setup and the recording rig is PX4, so it records only available=false; sensorsettings.rs sensor_settings_view serves it",
    "priority": "view.sensorSettings is ArduPilot's AHRS/compass setup and the recording rig is PX4, so it records only available=false; sensorsettings.rs sensor_settings_view serves it",
    "use": "view.sensorSettings is ArduPilot's AHRS/compass setup and the recording rig is PX4, so it records only available=false; sensorsettings.rs sensor_settings_view serves it",
    "confirmFirst": "view.apmSubFrame is ArduSub's frame page and the recording rig is a PX4 multirotor, so it records only available=false; apmsubframe.rs apm_sub_frame_view serves it",
    "frames": "view.apmSubFrame is ArduSub's frame page and the recording rig is a PX4 multirotor, so it records only available=false; apmsubframe.rs apm_sub_frame_view serves it",
    "hasDefaults": "view.apmSubFrame is ArduSub's frame page and the recording rig is a PX4 multirotor, so it records only available=false; apmsubframe.rs apm_sub_frame_view serves it",
    "loadError": "view.apmSubFrame is ArduSub's frame page and the recording rig is a PX4 multirotor, so it records only available=false; apmsubframe.rs apm_sub_frame_view serves it",
    "loadingDefaults": "view.apmSubFrame is ArduSub's frame page and the recording rig is a PX4 multirotor, so it records only available=false; apmsubframe.rs apm_sub_frame_view serves it",
    "autoDetectHelp": "view.apmSubMotors is ArduSub's motor page and the recording rig is a PX4 multirotor, so it records only available=false; apmsubmotors.rs apm_sub_motors_view serves it",
    "canRunManualTest": "view.apmSubMotors is ArduSub's motor page and the recording rig is a PX4 multirotor, so it records only available=false; apmsubmotors.rs apm_sub_motors_view serves it",
    "detecting": "view.apmSubMotors is ArduSub's motor page and the recording rig is a PX4 multirotor, so it records only available=false; apmsubmotors.rs apm_sub_motors_view serves it",
    "detectionMessages": "view.apmSubMotors is ArduSub's motor page and the recording rig is a PX4 multirotor, so it records only available=false; apmsubmotors.rs apm_sub_motors_view serves it",
    "motor": "view.apmSubMotors is ArduSub's motor page and the recording rig is a PX4 multirotor, so it records only available=false; apmsubmotors.rs apm_sub_motors_view serves it",
    "offersAutoDetect": "view.apmSubMotors is ArduSub's motor page and the recording rig is a PX4 multirotor, so it records only available=false; apmsubmotors.rs apm_sub_motors_view serves it",
    "address": "view.links bluetooth.devices lists the Android Bluetooth host's paired and scanned devices; the desktop recording rig installs no Bluetooth host, so the list records empty; platformbluetooth.rs links_field serves it",
    "enabledText": "view.joystick indicator is served only while a joystick is attached, and the desktop recording rig has none; joystickhost.rs indicator() serves it",
    "inputsText": "view.joystick indicator is served only while a joystick is attached, and the desktop recording rig has none; joystickhost.rs indicator() serves it",
    "typeText": "view.joystick indicator is served only while a joystick is attached, and the desktop recording rig has none; joystickhost.rs indicator() serves it",
    "warn": "view.joystick indicator is served only while a joystick is attached, and the desktop recording rig has none; joystickhost.rs indicator() serves it",
    "presetKind": "view.itemFacts is recorded under the Qt plan, and presetKind is served only by the core plan's pattern editor (coreplan.rs item facts for survey/corridor), which is what offers presets",
    "data": "view.opticalFlow image is served once a PX4Flow-style sensor has sent DATA_TRANSMISSION_HANDSHAKE/ENCAPSULATED_DATA, which the SITL rig never does; flowimage.rs serves it",
    "signedIn": "the account object is the core flavor's AircastAccount port (account.rs object()), served only without a Qt host, so the Qt-hosted recording rig never sees it",
    "signingIn": "the account object is the core flavor's AircastAccount port (account.rs object()), served only without a Qt host, so the Qt-hosted recording rig never sees it",
    "userCode": "the account object is the core flavor's AircastAccount port (account.rs object()), served only without a Qt host, so the Qt-hosted recording rig never sees it",
    "verificationUrl": "the account object is the core flavor's AircastAccount port (account.rs object()), served only without a Qt host, so the Qt-hosted recording rig never sees it",
    "depth": "view.logCategories lists the log targets the core logger has seen, and the Qt-hosted rig records it before any core target has logged, so the list records empty; applog.rs categories_view serves it",
    "shortName": "view.logCategories lists the log targets the core logger has seen, and the Qt-hosted rig records it before any core target has logged, so the list records empty; applog.rs categories_view serves it",
    "volume": "view.speech lists lines spoken since the given sequence, and the recording rig has said nothing when it records, so the list records empty; speech.rs speech_view serves volume per line",
    "yaw": "view.gimbalAzimuth lists gimbals with an absolute yaw, and the PX4 SITL rig has no gimbal, so the list records empty; gimbalindicator.rs azimuths serves yaw per gimbal",
    "batteryIndex": "the PX4 SITL rig simulates its battery without an analog voltage divider or current sensor, so Power records no Calculate rows; powercalc.rs calculator serves these on BATn_V_DIV/A_PER_V and ArduPilot's VOLT_MULT/AMP_PERVLT",
    "calculator": "the PX4 SITL rig simulates its battery without an analog voltage divider or current sensor, so Power records no Calculate rows; powercalc.rs calculator serves these on BATn_V_DIV/A_PER_V and ArduPilot's VOLT_MULT/AMP_PERVLT",
    "measure": "the PX4 SITL rig simulates its battery without an analog voltage divider or current sensor, so Power records no Calculate rows; powercalc.rs calculator serves these on BATn_V_DIV/A_PER_V and ArduPilot's VOLT_MULT/AMP_PERVLT",
    "measuredLabel": "the PX4 SITL rig simulates its battery without an analog voltage divider or current sensor, so Power records no Calculate rows; powercalc.rs calculator serves these on BATn_V_DIV/A_PER_V and ArduPilot's VOLT_MULT/AMP_PERVLT",
    "noReading": "the PX4 SITL rig simulates its battery without an analog voltage divider or current sensor, so Power records no Calculate rows; powercalc.rs calculator serves these on BATn_V_DIV/A_PER_V and ArduPilot's VOLT_MULT/AMP_PERVLT",
    "paramLabel": "the PX4 SITL rig simulates its battery without an analog voltage divider or current sensor, so Power records no Calculate rows; powercalc.rs calculator serves these on BATn_V_DIV/A_PER_V and ArduPilot's VOLT_MULT/AMP_PERVLT",
    "readingLabel": "the PX4 SITL rig simulates its battery without an analog voltage divider or current sensor, so Power records no Calculate rows; powercalc.rs calculator serves these on BATn_V_DIV/A_PER_V and ArduPilot's VOLT_MULT/AMP_PERVLT",
    "uploaded": "view.mavlinkLog lists the saved .ulg logs, and the recording rig's log folder is empty, so the list records empty; mavlinklog.rs files serves uploaded per file",
    "writing": "view.mavlinkLog lists the saved .ulg logs, and the recording rig's log folder is empty, so the list records empty; mavlinklog.rs files serves writing per file",
    "showUnits": "InstrumentDisplay.kt reads its own SharedPreferences JSON (per-value Telemetry Display styling), not a bridge view",
    "chart": "view.inspectorCharts lists plots and charted fields only after a field is put on a chart, and the recording rig charts nothing, so plots and selectedCharted record empty; inspectorchart.rs serves field and chart per plot",
    "field": "view.inspectorCharts lists plots and charted fields only after a field is put on a chart, and the recording rig charts nothing, so plots and selectedCharted record empty; inspectorchart.rs serves field and chart per plot",
    "singleStickDisplay": "view.joystick serves calibration only while a joystick calibration runs, and the recording rig has no joystick, so calibration records null; stickcal.rs and joystickhost.rs serve these during calibration",
    "initialConnectComplete": "ActiveVehicle.kt reads Vehicle's loadProgress and initialConnectComplete properties straight through get_fields(vehicle), not through a recorded view; vehiclefacade.rs serves both in the core flavor",
    "loadProgress": "ActiveVehicle.kt reads Vehicle's loadProgress and initialConnectComplete properties straight through get_fields(vehicle), not through a recorded view; vehiclefacade.rs serves both in the core flavor",
    "keepText": "view.plan carries vehicleChangePrompt only after the active vehicle changes under a dirty plan, which the recording rig never does; coreplan.rs vehicle_change_prompt serves these",
    "loadText": "view.plan carries vehicleChangePrompt only after the active vehicle changes under a dirty plan, which the recording rig never does; coreplan.rs vehicle_change_prompt serves these",
    "details": "view.joystick indicator is served only while a joystick is attached, and the desktop recording rig has none; joystickhost.rs indicator() serves the detail rows",
    "showIcon": "InstrumentDisplay.kt reads its own SharedPreferences JSON (per-value Telemetry Display styling), not a bridge view",
    "icon": "InstrumentDisplay.kt reads its own SharedPreferences JSON (per-value Telemetry Display styling), not a bridge view",
    "rangeType": "InstrumentDisplay.kt reads its own SharedPreferences JSON (per-value Telemetry Display styling), not a bridge view",
    "optional": "view.itemFacts marks nanFacts fields optional, and the recording plan has no VTOL land or DO_SET_ACTUATOR item to carry one; coreplan.rs simple_fields and itemfacts.rs listed serve it",
    "chosen": "view.apmAirframe is ArduPilot copter/rover's frame page and the recording rig is a PX4 multirotor, so it records only available=false; apmairframe.rs apm_airframe_view serves it",
    "classes": "view.apmAirframe is ArduPilot copter/rover's frame page and the recording rig is a PX4 multirotor, so it records only available=false; apmairframe.rs apm_airframe_view serves it",
    "frameType": "view.apmAirframe is ArduPilot copter/rover's frame page and the recording rig is a PX4 multirotor, so it records only available=false; apmairframe.rs apm_airframe_view serves it",
    "invalidText": "view.apmAirframe is ArduPilot copter/rover's frame page and the recording rig is a PX4 multirotor, so it records only available=false; apmairframe.rs apm_airframe_view serves it",
    "orientationTitle": "view.sensorSettings lists only external mags with a settable rotation, and the PX4 SITL rig's mags report CAL_MAGn_ROT -1, so the list records empty; sensorsettings.rs serves it per mag",
    "timestamp": "view.appLog is recorded under Qt, where the core logger holds no entries; applog.rs serves it",
    "errorMessage": "GeoTagView.kt reads the geoTag controller object itself (geotagcontroller.rs object()), not a recorded view",
    "failedCount": "GeoTagView.kt reads the geoTag controller object itself (geotagcontroller.rs object()), not a recorded view",
    "imageDirectory": "GeoTagView.kt reads the geoTag controller object itself (geotagcontroller.rs object()), not a recorded view",
    "logFile": "GeoTagView.kt reads the geoTag controller object itself (geotagcontroller.rs object()), not a recorded view",
    "previewMode": "GeoTagView.kt reads the geoTag controller object itself (geotagcontroller.rs object()), not a recorded view",
    "saveDirectory": "GeoTagView.kt reads the geoTag controller object itself (geotagcontroller.rs object()), not a recorded view",
    "skippedCount": "GeoTagView.kt reads the geoTag controller object itself (geotagcontroller.rs object()), not a recorded view",
    "taggedCount": "GeoTagView.kt reads the geoTag controller object itself (geotagcontroller.rs object()), not a recorded view",
    "timeOffsetSecs": "GeoTagView.kt reads the geoTag controller object itself (geotagcontroller.rs object()), not a recorded view",
    "channelHint": "view.syslink serves only available=false on the PX4 SITL rig, which has no SLNK_RADIO_* parameters (a Crazyflie does); syslink.rs syslink_view serves it",
    "addressHint": "view.syslink serves only available=false on the PX4 SITL rig, which has no SLNK_RADIO_* parameters (a Crazyflie does); syslink.rs syslink_view serves it",
    "rates": "view.syslink serves only available=false on the PX4 SITL rig, which has no SLNK_RADIO_* parameters (a Crazyflie does); syslink.rs syslink_view serves it",
    "address": "view.syslink serves only available=false on the PX4 SITL rig, which has no SLNK_RADIO_* parameters (a Crazyflie does); syslink.rs syslink_view serves it",
}


def main():
    if not os.path.exists(CONTRACT):
        print(f"no contract at {CONTRACT}; set VIEW_SHAPES", file=sys.stderr)
        return 2
    contract = served(json.load(open(CONTRACT)), set())
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    reads = head_keys(root)
    if not reads:
        print(f"  REFUSING: no .kt reads found under {root}, so every count below would be meaningless")
        return 1
    missing = {k: v for k, v in sorted(reads.items()) if k not in contract}

    for control, expected in [("altitudeFrame", True), ("foldedCommands", True), ("notAKeyAnyViewServes", False)]:
        present = control in contract
        if present != expected:
            print(f"CONTROL FAILED: {control} in contract = {present}, expected {expected}")
            return 3

    escaped = {
        k: sorted(v - ONLY_IN[k])
        for k, v in missing.items()
        if k in ONLY_IN and v - ONLY_IN[k]
    }
    unexplained = {k: v for k, v in missing.items() if k not in ACCEPTED}
    for key, files in sorted(escaped.items()):
        print(f"  ACCEPTED ELSEWHERE: {key} is exempt in {', '.join(sorted(ONLY_IN[key]))}, "
              f"but {', '.join(files)} reads it too - a different object with the same key name")
    stale = [k for k in ACCEPTED if k not in missing]
    print(f"{len(contract)} keys in the contract, {len(reads)} read by the head, "
          f"{len(missing)} not served, {len(unexplained)} unexplained")
    for key in stale:
        print(f"  STALE ACCEPTANCE: {key} is served now; delete its entry")
    for key, files in unexplained.items():
        print(f"  NOT SERVED: {key:<24} {', '.join(sorted(files))}")

    blind = recorded_empty(json.load(open(CONTRACT)), "", [])
    print(f"\n  {len(blind)} list(s) recorded empty. Their element fields are in no contract,")
    print("  so the core can rename or drop one and every check here stays green.")
    for where in sorted(blind):
        print(f"    EMPTY WHEN RECORDED: {where}")

    tree = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

    unpinned = sorted(recorded_null(json.load(open(CONTRACT)), set()) & opened_as_container(tree))
    if unpinned:
        print(f"\n  {len(unpinned)} key(s) recorded NULL that the head opens as an object or array:")
        print(f"    {', '.join(unpinned)}")
        print("  Their children are in no contract either, for the same reason as the empty lists.")

    drawn = head_views(tree)

    unknown = sorted(drawn - {k.split("(")[0] for k in json.load(open(CONTRACT))} - not_recorded(tree))
    if unknown:
        print(f"\n  {len(unknown)} view path(s) this head names that the core does not serve:")
        print(f"    {', '.join(unknown)}")
        print("  A typo here reads as a view that exists and has nothing to say.")
    if not drawn:
        print("  REFUSING: no view path found in any .kt, so every shape would look unconsumed")
        return 1

    shapes = []
    for key, value in json.load(open(CONTRACT)).items():
        groups(value, key.split("(")[0], shapes)

    beside, absent = {}, {}
    for root, names in shapes:
        known = names & set(reads)
        if not known:
            continue
        target = beside if root in drawn else absent
        for name in sorted(names - set(reads) - ACCEPTED.keys()):
            if (root, name) in ACCEPTED_ON:
                continue
            target.setdefault(name, set()).add(root)
    absent = {k: v for k, v in absent.items() if k not in beside}

    served_on = {(root, n) for root, names in shapes for n in names}
    for key in sorted(ACCEPTED_ON):
        if key not in served_on:
            print(f"  STALE ACCEPTANCE: {key[0]} no longer serves {key[1]}; delete its entry")
        elif key[1] in reads:
            print(f"  STALE ACCEPTANCE: {key[1]} is read now; delete the {key[0]} entry")

    seen = set()
    if os.path.exists(BASELINE):
        seen = {line.strip() for line in open(BASELINE) if line.strip()}
    fresh = {k: v for k, v in beside.items() if k not in seen}
    if "--baseline" in sys.argv:
        with open(BASELINE, "w") as handle:
            handle.write("\n".join(sorted(set(beside) | set(absent))) + "\n")
        print(f"\n  baseline written: {len(beside)} beside a read view, {len(absent)} in views with no screen")
        return 0
    if fresh:
        print(f"\n  {len(fresh)} field(s) NEWLY served beside ones this head already reads:")
        for name, near in sorted(fresh.items()):
            print(f"    UNREAD BESIDE {', '.join(sorted(near))}: {name}")
        print("  A field the core adds to a shape you consume is invisible to every other check here.")

    unbuilt = {k: v for k, v in absent.items() if k not in seen}
    if unbuilt:
        views = sorted({r for roots in unbuilt.values() for r in roots})
        print(f"\n  {len(unbuilt)} field(s) in {len(views)} view(s) this head has no screen for:")
        print(f"    {', '.join(views)}")
        print("  These are whole features, not fields missed beside ones we read - a different call.")
    uncommitted = fixture_provenance()
    if uncommitted:
        print(f"\n  UNCOMMITTED FIXTURE: {uncommitted} differs from HEAD.")
        print("  Every count above includes keys recorded from a producer that may not be committed")
        print("  either. Verify with git show HEAD:<path> before calling any field served.")

    nullable = nullable_bools(json.load(open(CONTRACT)), "", {})
    if "contactLost" not in nullable:
        print("  REFUSING: the contract parse found no bool|null fields, so an empty result means nothing")
        return 1
    flat = flattened(os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), nullable)
    if flat:
        print(f"\n  {len(flat)} field(s) the core can serve as null, read as a bare Boolean:")
        for name, where in sorted(flat.items()):
            print(f"    FLATTENED {name}: {', '.join(sorted(where))} - contract says {sorted(nullable[name])[0]} is bool|null")
        print("  optBoolean turns JSON null into false, and false is the reassuring answer every time.")

    def plain_bools(node, root, into):
        if isinstance(node, dict):
            for key, value in node.items():
                if value == "bool":
                    into.add((root, key))
                else:
                    plain_bools(value, root, into)
        elif isinstance(node, list):
            for item in node:
                plain_bools(item, root, into)
        return into

    declared = set()
    for key, value in json.load(open(CONTRACT)).items():
        if not key.startswith("_"):
            plain_bools(value, key.split("(")[0], declared)
    declared_paths = {f"{root}.{field}" for root, field in declared}
    witness = json.load(open(CONTRACT))
    never = set(witness.get("_neverVaried", []))
    bare = set()
    for base, _, names in os.walk(tree):
        if "/build/" in base or "/test/" in base:
            continue
        for name in names:
            if name.endswith(".kt"):
                body = open(os.path.join(base, name)).read()
                bare |= {k for k in BARE_BOOL.findall(body) if f'isNull("{k}")' not in body}
    unwitnessed = set(witness.get("view.contract", {}).get("nullableUnwitnessed", []))
    lying = sorted(p for p in unwitnessed if p.rsplit(".", 1)[-1] in bare)
    if lying:
        print(f"\n  {len(lying)} field(s) the core can serve as null, read here as a bare Boolean:")
        for p in lying:
            print(f"    UNWITNESSED NULL {p}")
        print("  The contract types these bool because the rig never produced the null, so the")
        print("  flattened-bool check above cannot see them. Read them with isNull, not optBoolean.")

    exposed = sorted(p for p in declared_paths if p.rsplit(".", 1)[-1] in bare and p in never)
    print(f"\n  {len(declared)} view/field pair(s) are typed bool from observation alone,")
    print(f"  {len(exposed)} of them read here with a bare optBoolean and never seen to vary.")
    print("  That is a risk population, not a defect list: the rig has one vehicle in one state,")
    print("  so most never varied for that reason. view.contract.nullableUnwitnessed separates")
    print(f"  the ones the core can actually serve as null - it names {len(unwitnessed)} path(s) today, and")
    print("  it is hand-maintained, so a clean section above is a lower bound, not an audit.")

    return 1 if unexplained or stale or fresh or flat or escaped or unknown or lying else 0


if __name__ == "__main__":
    sys.exit(main())
