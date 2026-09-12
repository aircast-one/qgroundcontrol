import Foundation

struct CameraControl: Equatable {
    static let photoMode = 0
    static let videoMode = 1

    let present: Bool
    let title: String
    let model: String
    let mode: Int
    let modeKnown: Bool
    let modeText: String
    let isRecording: Bool
    let isTakingPhoto: Bool
    let stateText: String
    let clockText: String
    let storageText: String
    let shotsText: String
    let batteryText: String
    let hasZoom: Bool
    let zoomLevel: Double
    let canRecord: Bool
    let canPhoto: Bool
    let hasModes: Bool
    let canChangeMode: Bool

    var offersModePicker: Bool {
        mode == CameraControl.photoMode || mode == CameraControl.videoMode
    }

    static let modeBusy = "The camera is capturing. It will not change mode until that finishes."

    static let absent = CameraControl()

    private init() {
        present = false
        title = ""
        model = ""
        mode = -1
        modeKnown = false
        modeText = ""
        isRecording = false
        isTakingPhoto = false
        stateText = ""
        clockText = ""
        storageText = ""
        shotsText = ""
        batteryText = ""
        hasZoom = false
        zoomLevel = 1
        canRecord = false
        canPhoto = false
        hasModes = false
        canChangeMode = false
    }

    init(_ json: [String: Any]) {
        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }
        func text(_ name: String) -> String { (json[name] as? String) ?? "" }
        func number(_ name: String, _ fallback: Int) -> Int {
            (json[name] as? NSNumber)?.intValue ?? fallback
        }
        present = flag("present")
        title = text("title")
        model = text("model")
        mode = number("mode", -1)
        modeKnown = flag("modeKnown")
        modeText = text("modeText")
        isRecording = flag("isRecording")
        isTakingPhoto = flag("isTakingPhoto")
        stateText = text("stateText")
        clockText = text("clockText")
        storageText = text("storageText")
        shotsText = text("shotsText")
        batteryText = text("batteryText")
        hasZoom = flag("hasZoom")
        zoomLevel = (json["zoomLevel"] as? NSNumber)?.doubleValue ?? 1
        canRecord = flag("canRecord")
        canPhoto = flag("canPhoto")
        hasModes = flag("hasModes")
        canChangeMode = flag("canChangeMode")
    }

    // One place decides whether each control exists, so the store's guard and the view's
    // condition cannot drift apart. Both are the core's answer, not a re-reading of the mode.
    var offersShutter: Bool { present && canPhoto }
    var offersRecord: Bool { present && canRecord }
}
