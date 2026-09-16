"""Which Swift model decodes which view, shared by the two checkers that need it.

view-fields.py asks whether every key a model reads is one the core still emits.
null-fallbacks.py asks what a model does with the keys the core emits as null. Both
need the same map, and a map written twice is one a new model gets added to once.
"""

MODELS = {
    "AltitudeModeOffer": "view.altitudeModes",
    "Detections": "view.detections",
    "EditablePolygon": "view.polygon(plan.missionController.visualItems.4.surveyAreaPolygon)",
    "GuidedRange": ["view.guidedAltitude(30)", "view.guidedTakeoff(10)",
                    "view.guidedSpeed(3)"],
    "InstrumentValue": "view.instruments(Altitude)",
    "MissionSeed": "view.missionSeed(survey,47.397,8.545)",
    "SettingsPage": "view.settings",
    "SettingsSection": "view.settings(General)",
    "VehicleMessages": "view.messages",
    "VehicleTrack": "view.track",
    "VehicleWarning": "view.warnings",
    "BatteryReading": "view.battery",
    "CalibrationState": "view.calibration",
    "CameraControl": "view.camera",
    "FollowMe": "view.followMe",
    "ItemCamera": "view.itemCamera",
    "Fleet": "view.vehicles",
    "FenceShape": "view.fences",
    "AdsbTraffic": "view.adsbTraffic",
    "FlyState": "view.flyState",
    "FrameSetup": "view.frame",
    "MotorTest": "view.frame",
    "FactRange": "view.control(settings.appSettings.defaultMissionItemAltitude)",
    "InstrumentGroup": "view.instrumentGroups",
    "LinkFormCheck": "view.linkForm(udp,127.0.0.1,14550)",
    "RemoteSupport": "view.supportHost(support.ardupilot.org:14550)",
    "MavlinkConsole": "view.mavlinkConsole",
    "Orbit": "view.orbit",
    "ObstacleDistance": "view.obstacle",
    "TlogSummary": "view.tlog(/tmp/none.tlog)",
    "JoystickMapping": "view.joystickMapping",
    "PacketRadio": "view.packetRadio",
    "FlightModeChoice": "view.flightModes",
    "LandingPattern": "view.landingPattern(6)",
    "LinkConfig": "view.links",
    "LogEntry": "view.logs",
    "GcsFix": "view.gcsPosition",
    "GuidedOffer": "view.guidedActions",
    "VehicleLink": "view.vehicleLinks",
    "MavlinkMessage": "view.inspector",
    "MissionItem": "view.missionItems",
    "MissionItemKind": "view.missionKinds",
    "MissionSummary": "view.missionSummary",
    "MapScaleBar": "view.mapScale(120)",
    "ModeSlot": "view.modeSlots",
    "ModeSlots": "view.modeSlots",
    # Both decode a SUB-dictionary of view.plan rather than its top level, which the checker
    # tolerates because it compares key names against the whole of plan.rs rather than against a
    # nesting. So this catches a key the core DROPS and would not catch one it MOVES between
    # levels -- weaker than the check a top-level model gets, and worth more than the nothing
    # view.plan had, which was nothing because MODELS is keyed by type and Mission.swift decodes
    # the rest of this view in place.
    "PlanReadiness": "view.plan",
    "PlanUpload": "view.plan",
    "PreflightCheck": "view.preflight",
    "RadioState": "view.radio",
    "SensorHealth": "view.sensors",
    "SurveyStats": "view.surveyStats",
    "TerrainProfile": "view.terrainProfile",
    "VehicleComponentInfo": "view.setup",
    "VibrationReading": "view.vibration",
    "VideoStatus": "view.video",
}


def views(declared):
    return list(declared) if isinstance(declared, (list, tuple)) else [declared]
