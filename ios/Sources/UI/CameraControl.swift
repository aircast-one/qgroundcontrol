import Foundation

let CAM_MODE_PHOTO = 0
let CAM_MODE_VIDEO = 1

struct CameraShutter: Equatable {
    var label: String
    var recording: Bool
    var enabled: Bool
    var action: String?
    var video: Bool
    var readout: String
    var readoutActive: Bool
}

struct CameraPanel: Equatable {
    var visible: Bool
    var inPhotoMode: Bool
    var selectVideoEnabled: Bool
    var selectPhotoEnabled: Bool
    var bothShown: Bool
    var shutters: [CameraShutter]
    var freeText: String?
    var batteryText: String?
}

private func objectAt(_ json: JSON, _ key: String) -> JSON? {
    json[key].object != nil ? json[key] : nil
}

private func nonBlank(_ text: String) -> String? { text.isBlank ? nil : text }

func cameraPanel(_ panel: JSON?) -> CameraPanel? {
    guard let panel else { return nil }
    let video = objectAt(panel, "video").map { v in
        CameraShutter(
            label: v["capturing"].bool ? "Stop recording" : "Start recording",
            recording: v["capturing"].bool,
            enabled: v["enabled"].bool,
            action: CAMERA_RECORD,
            video: true,
            readout: v["clock"].string,
            readoutActive: !v["idle"].bool
        )
    }
    let photo = objectAt(panel, "photo").map { p in
        let press = p["press"].string
        return CameraShutter(
            label: press == "stop" ? "Stop photos" : "Take photo",
            recording: p["capturing"].bool,
            enabled: p["enabled"].bool,
            action: press == "stop" ? CAMERA_STOP_PHOTO : press == "take" ? CAMERA_PHOTO : nil,
            video: false,
            readout: p["count"].string,
            readoutActive: !p["idle"].bool
        )
    }
    return CameraPanel(
        visible: panel["visible"].bool,
        inPhotoMode: panel["inPhotoMode"].bool,
        selectVideoEnabled: panel["selectVideoEnabled"].bool,
        selectPhotoEnabled: panel["selectPhotoEnabled"].bool,
        bothShown: panel["bothShown"].bool,
        shutters: [video, photo].compactMap { $0 },
        freeText: nonBlank(panel["freeText"].string),
        batteryText: nonBlank(panel["batteryText"].string)
    )
}

func modeTapSwitches(_ camera: CameraReading, _ video: Bool) -> Bool {
    camera.mode != (video ? CAM_MODE_VIDEO : CAM_MODE_PHOTO)
}

func shutterCaption(_ panel: CameraPanel, _ shutter: CameraShutter) -> String? {
    panel.bothShown ? (shutter.video ? "Video" : "Photo") : nil
}

let CAMERA_VIEW = "view.camera"

struct CameraReading: Equatable {
    var hasModes: Bool
    var canChangeMode: Bool
    var modeText: String
    var isRecording: Bool
    var canPhoto: Bool
    var canRecord: Bool
    var isTakingPhoto: Bool
    var capturesPhotos: Bool
    var hasVideoStream: Bool = false
    var mode: Int
    var timelapse: Bool
    var lapseSeconds: Double?
    var lapseCount: Int?
    var lapseUnlimited: Bool
    var title: String
    var labels: [String]
    var stateText: String
    var reportsStorage: Bool
    var storageText: String
    var shotsText: String
    var batteryText: String
    var hasZoom: Bool
    var zoomLevel: Double
    var selected: Int? = nil
    var streamLabels: [String] = []
    var currentStream: Int = 0
    var panel: CameraPanel? = nil
}

func cameraReading(_ view: JSON?) -> CameraReading? {
    guard let view, view["present"].bool else { return nil }
    return CameraReading(
        hasModes: view["hasModes"].bool,
        canChangeMode: view["canChangeMode"].bool,
        modeText: view["modeText"].string,
        isRecording: view["isRecording"].bool,
        canPhoto: view["canPhoto"].bool,
        canRecord: view["canRecord"].bool,
        isTakingPhoto: view["isTakingPhoto"].bool,
        capturesPhotos: view["capturesPhotos"].bool,
        hasVideoStream: view["hasVideoStream"].bool,
        mode: view["mode"].int(-1),
        timelapse: view["photoMode"].string == "timelapse",
        lapseSeconds: view["lapseSeconds"].double.flatMap { $0.isFinite ? $0 : nil },
        lapseCount: view.has("lapseCount") ? view["lapseCount"].int(0) : nil,
        lapseUnlimited: view["lapseUnlimited"].bool,
        title: view["title"].string,
        labels: view["labels"].array.map(\.string),
        stateText: view["stateText"].string,
        reportsStorage: view["reportsStorage"].bool,
        storageText: view["storageText"].string,
        shotsText: view["shotsText"].string,
        batteryText: view["batteryText"].string,
        hasZoom: view["hasZoom"].bool,
        zoomLevel: view["zoomLevel"].double.flatMap { $0.isFinite ? $0 : nil } ?? ZOOM_LOWEST,
        selected: view.has("selected") ? view["selected"].int(0) : nil,
        streamLabels: view["streamLabels"].array.map(\.string),
        currentStream: view["currentStream"].int(0),
        panel: cameraPanel(objectAt(view, "panel"))
    )
}

let ZOOM_LOWEST = 0.0
let ZOOM_HIGHEST = 100.0
let CAMERA_ZOOM = "vehicle.cameraManager.currentCameraInstance.zoomLevel"
let CAMERA_START_TRACKING = "vehicle.cameraManager.currentCameraInstance.startTracking"
let CAMERA_STOP_TRACKING = "vehicle.cameraManager.currentCameraInstance.stopTracking"

struct TrackingBox: Equatable {
    var x: Double
    var y: Double
    var width: Double
    var height: Double
}

struct TrackingReading: Equatable {
    var supported: Bool
    var requested: Bool
    var reported: Bool
    var shapes: [String]
    var box: TrackingBox?
}

func trackingReading(_ view: JSON?) -> TrackingReading? {
    guard let tracking = view.flatMap({ objectAt($0, "tracking") }), tracking["supported"].bool else { return nil }
    let box = objectAt(tracking, "rect").map { rect in
        TrackingBox(x: rect["x"].double(.nan), y: rect["y"].double(.nan), width: rect["width"].double(.nan), height: rect["height"].double(.nan))
    }
    return TrackingReading(
        supported: true,
        requested: tracking["requested"].bool,
        reported: tracking["reported"].bool,
        shapes: tracking["shapes"].array.compactMap { nonBlank($0.string) },
        box: box.flatMap { $0.width > 0 && $0.height > 0 ? $0 : nil }
    )
}

let CAMERA_TRACKING_ARMED = "vehicle.cameraManager.currentCameraInstance.trackingEnabled"

func trackingToggleLabel(_ reading: TrackingReading) -> String {
    reading.requested ? "Stop tracking" : "Track something"
}

func trackingCanAim(_ reading: TrackingReading?) -> Bool {
    reading.map { $0.requested && !$0.shapes.isEmpty } ?? false
}

func trackingCanStart(_ reading: TrackingReading?) -> Bool {
    reading.map { !$0.requested && !$0.shapes.isEmpty } ?? false
}

func trackingCanStop(_ reading: TrackingReading?) -> Bool {
    reading?.requested ?? false
}

func trackingRectObject(_ box: TrackingBox) -> [String: Double] {
    ["x": box.x, "y": box.y, "width": box.width, "height": box.height]
}

let TRACK_POINT_SLOP_DP = 10.0
let TRACK_POINT_RADIUS_DP = 50.0

enum TrackingRequest: Equatable {
    case Box(rect: TrackingBox)
    case Point(x: Double, y: Double, radius: Double)
}

func trackingRequest(_ pressX: Double, _ pressY: Double, _ releaseX: Double, _ releaseY: Double, _ picture: PaintedRect) -> TrackingRequest? {
    guard picture.width > 0, picture.height > 0 else { return nil }
    let acrossX = { (value: Double) in min(max((value - picture.left) / picture.width, 0), 1) }
    let acrossY = { (value: Double) in min(max((value - picture.top) / picture.height, 0), 1) }
    let x0 = acrossX(min(pressX, releaseX))
    let x1 = acrossX(max(pressX, releaseX))
    let y0 = acrossY(min(pressY, releaseY))
    let y1 = acrossY(max(pressY, releaseY))
    let tapped = abs(releaseX - pressX) < TRACK_POINT_SLOP_DP && abs(releaseY - pressY) < TRACK_POINT_SLOP_DP
    if tapped { return .Point(x: x0, y: y0, radius: TRACK_POINT_RADIUS_DP / picture.width) }
    if x1 - x0 <= 0 || y1 - y0 <= 0 { return nil }
    return .Box(rect: TrackingBox(x: x0, y: y0, width: x1 - x0, height: y1 - y0))
}

func trackingPointObject(_ x: Double, _ y: Double) -> [String: Double] {
    ["x": x, "y": y]
}

let TRACKING_CENTRE = TrackingBox(x: 0.4, y: 0.4, width: 0.2, height: 0.2)

let CAMERA_FORMAT = "vehicle.cameraManager.currentCameraInstance.formatCard"

struct DestructiveAction: Equatable, Identifiable {
    var id: String
    var label: String
    var button: String
    var title: String
    var prompt: String
    var offer: String
    var reason: String

    var ready: Bool { offer == "ready" }
    var blocked: Bool { offer == "blocked" }
    var shown: Bool { offer != "hidden" }
}

func destructiveActions(_ view: JSON?) -> [DestructiveAction] {
    guard let listed = view?["destructiveActions"].arrayOrNil else { return [] }
    return listed.filter { $0.object != nil }.compactMap { entry in
        nonBlank(entry["id"].string).map { id in
            DestructiveAction(
                id: id,
                label: entry["label"].string,
                button: entry["button"].string,
                title: entry["title"].string,
                prompt: entry["prompt"].string,
                offer: entry["offer"].string,
                reason: entry["reason"].string
            )
        }
    }.filter(\.shown)
}

func destructiveReasonFor(_ action: DestructiveAction) -> String? {
    action.blocked ? nonBlank(action.reason) : nil
}

func destructiveInvokePath(_ id: String) -> String? {
    switch id {
    case "resetSettings": CAMERA_RESET
    case "formatStorage": CAMERA_FORMAT
    default: nil
    }
}

let CAMERA_PHOTO_MODE = "vehicle.cameraManager.currentCameraInstance.photoCaptureMode"
let CAMERA_PHOTO_LAPSE = "vehicle.cameraManager.currentCameraInstance.photoLapse"
let PHOTO_LAPSE_MIN_S = 1.0
let PHOTO_LAPSE_MAX_S = 60.0
let CAMERA_CURRENT_STREAM = "vehicle.cameraManager.currentCameraInstance.currentStream"
let CAMERA_THERMAL_MODE = "vehicle.cameraManager.currentCameraInstance.thermalMode"
let CAMERA_SHEET_VIDEO_SETTINGS: KeyValuePairs<String, String> = [
    "settings.videoSettings.gridLines": "Video Grid Lines",
    "settings.videoSettings.videoFit": "Video Screen Fit",
]
let CAMERA_THERMAL_OPACITY = "vehicle.cameraManager.currentCameraInstance.thermalOpacity"

let THERMAL_MODES = ["off", "blend", "full", "picInPic"]

func thermalModeLabel(_ token: String) -> String {
    switch token {
    case "off": "Off"
    case "blend": "Blend"
    case "full": "Full"
    case "picInPic": "Picture in picture"
    default: token
    }
}

struct ThermalReading: Equatable {
    var mode: String
    var opacity: Double?
}

func thermalReading(_ view: JSON?) -> ThermalReading? {
    guard let view, view["thermalAvailable"].bool, let mode = nonBlank(view["thermalMode"].string) else { return nil }
    let opacity = view.has("thermalOpacity") ? view["thermalOpacity"].double : nil
    return ThermalReading(mode: mode, opacity: opacity.flatMap { $0.isFinite ? $0 : nil })
}

func thermalOpacityIsOffered(_ reading: ThermalReading?) -> Bool {
    reading?.opacity != nil
}

let CAMERA_RESET = "vehicle.cameraManager.currentCameraInstance.resetSettings"

func zoomText(_ camera: CameraReading) -> String? {
    camera.hasZoom ? "\(Int(camera.zoomLevel))%" : nil
}

func shutterReadout(_ shutter: CameraShutter) -> String? {
    guard shutter.readoutActive else { return nil }
    return shutter.video ? "REC \(shutter.readout)" : shutter.readout
}

let CAMERA_PHOTO = "camera.takePhoto"
let CAMERA_RECORD = "camera.toggleRecording"
let CAMERA_STOP_PHOTO = "camera.stopPhoto"
let CAMERA_SET_MODE = "camera.setMode"

func cameraDetails(_ camera: CameraReading) -> [(String, String)] {
    [
        ("State", camera.stateText),
        camera.reportsStorage && !camera.storageText.isBlank ? ("Storage", camera.storageText) : nil,
        !camera.shotsText.isBlank ? ("Photos", camera.shotsText) : nil,
        !camera.batteryText.isBlank ? ("Battery", camera.batteryText) : nil,
    ].compactMap { $0 }
}

func lapsePlan(_ camera: CameraReading) -> String? {
    guard camera.timelapse else { return nil }
    let every = camera.lapseSeconds.flatMap { $0 > 0 ? "every \(trimmed($0)) s" : nil }
    let many = camera.lapseUnlimited ? "until stopped" : camera.lapseCount.flatMap { $0 > 0 ? "\($0) shots" : nil }
    return [every, many].compactMap { $0 }.joined(separator: ", ").ifBlank("interval capture")
}

private func trimmed(_ value: Double) -> String {
    value == Double(Int64(value)) ? String(Int64(value)) : String(format: "%.1f", value)
}
