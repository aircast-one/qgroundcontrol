import Foundation

struct CameraControl: Equatable {
    static let photoMode = 0
    static let videoMode = 1

    let present: Bool
    let title: String
    let model: String
    let vendor: String
    let mode: Int
    let modeKnown: Bool
    let modeText: String
    let isRecording: Bool
    let isTakingPhoto: Bool
    let stateText: String
    let clockText: String
    let storageStatus: Int
    let storageText: String
    let shots: Int
    let shotsText: String
    let batteryRemaining: Int
    let batteryText: String
    let hasZoom: Bool
    let zoomLevel: Double
    let canRecord: Bool
    let canPhoto: Bool
    let hasModes: Bool
    let canChangeMode: Bool

    static let modeBusy = "The camera is capturing. It will not change mode until that finishes."

    static let absent = CameraControl()

    private init() {
        present = false
        title = ""
        model = ""
        vendor = ""
        mode = -1
        modeKnown = false
        modeText = ""
        isRecording = false
        isTakingPhoto = false
        stateText = ""
        clockText = ""
        storageStatus = 0
        storageText = ""
        shots = 0
        shotsText = ""
        batteryRemaining = -1
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
        vendor = text("vendor")
        mode = number("mode", -1)
        modeKnown = flag("modeKnown")
        modeText = text("modeText")
        isRecording = flag("isRecording")
        isTakingPhoto = flag("isTakingPhoto")
        stateText = text("stateText")
        clockText = text("clockText")
        storageStatus = number("storageStatus", 0)
        storageText = text("storageText")
        shots = number("shots", 0)
        shotsText = text("shotsText")
        batteryRemaining = number("batteryRemaining", -1)
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
