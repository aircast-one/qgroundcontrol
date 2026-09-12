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
    "CalibrationState": "view.calibration",
    "CameraControl": "view.camera",
    "FenceShape": "view.fences",
    "FlyState": "view.flyState",
    "FlightModeChoice": "view.flightModes",
    "LinkConfig": "view.links",
    "LogEntry": "view.logs",
    "GuidedOffer": "view.guidedActions",
    "MavlinkMessage": "view.inspector",
    "MissionItem": "view.missionItems",
    "MissionItemKind": "view.missionKinds",
    "MissionSummary": "view.missionSummary",
    "MapScaleBar": "view.mapScale(120)",
    "ModeSlot": "view.modeSlots",
    "ModeSlots": "view.modeSlots",
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
