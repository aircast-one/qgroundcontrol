"""Which Swift model decodes which view, shared by the two checkers that need it.

view-fields.py asks whether every key a model reads is one the core still emits.
null-fallbacks.py asks what a model does with the keys the core emits as null. Both
need the same map, and a map written twice is one a new model gets added to once.
"""

MODELS = {
    "AltitudeModeOffer": "view.altitudeModes",
    "CalibrationState": "view.calibration",
    "CameraControl": "view.camera",
    "FenceShape": "view.fences",
    "FlightModeChoice": "view.flightModes",
    "LinkConfig": "view.links",
    "LogEntry": "view.logs",
    "MavlinkMessage": "view.inspector",
    "MissionItem": "view.missionItems",
    "MissionItemKind": "view.missionKinds",
    "MissionSummary": "view.missionSummary",
    "PreflightCheck": "view.preflight",
    "RadioState": "view.radio",
    "SensorHealth": "view.sensors",
    "SurveyStats": "view.surveyStats",
    "TerrainProfile": "view.terrainProfile",
    "VehicleComponentInfo": "view.setup",
    "VibrationReading": "view.vibration",
    "VideoStatus": "view.video",
}
