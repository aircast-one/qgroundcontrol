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
    let labels: [String]
    let canPhoto: Bool
    let hasModes: Bool
    let canChangeMode: Bool

    // PhotoVideoControl.qml:115 offers the toggle on hasModes alone, and the core's
    // canChangeMode says a camera sitting in a third mode WILL accept the change whenever video
    // capture is stopped. The picker is the head's only route to setCameraMode, so gating its
    // existence on the current mode stranded such a camera with no way back to photo or video.
    var offersModePicker: Bool { hasModes }

    // The picker carries a tag for photo and one for video, so it cannot represent a third mode.
    // Where it cannot, the row says which mode the camera is actually in beside it.
    var pickerShowsCurrentMode: Bool {
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
        labels = []
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
        labels = ((json["labels"] as? [Any]) ?? []).compactMap { $0 as? String }
        canPhoto = flag("canPhoto")
        hasModes = flag("hasModes")
        canChangeMode = flag("canChangeMode")
    }

    // One place decides whether each control exists, so the store's guard and the view's
    // condition cannot drift apart. Both are the core's answer, not a re-reading of the mode.
    var offersShutter: Bool { present && canPhoto }
    var offersRecord: Bool { present && canRecord }
}

// The core answers every camera command now, and answering is the whole point: QGC's
// VehicleCameraControl refuses on terms it never reports, so a head that fires and forgets shows
// the operator exactly what a success shows them -- nothing. Three sentences were being computed
// and thrown away here.
//
// The three cases are not two. A refusal carries a reason and is the camera speaking. An answer
// with no `ok` at all is nobody speaking: the call did not reach the core's camera handler, which
// is what an older core or a renamed action looks like, and reporting that as a camera refusal
// would invent a fact about the hardware. Silence is the one thing it must not come back as,
// because silence is what the defect looked like.
enum CameraRefusal {
    static let unanswered = "The camera command was not answered."

    static func sentence(_ answer: [String: Any]) -> String? {
        guard let ok = (answer["ok"] as? NSNumber)?.boolValue else { return unanswered }
        guard !ok else { return nil }
        let reason = (answer["reason"] as? String) ?? ""
        return reason.isEmpty ? unanswered : reason
    }
}


// THERE IS DELIBERATELY NO ZoomAnswer DECODER, and the core's `clamped` is not being ignored.
//
// The zoom range is not a camera property, it is the MAVLink protocol: setZoomLevel sends
// ZOOM_TYPE_RANGE, whose level is a PERCENTAGE. So 0...100 is the same number in three places on
// purpose -- this head's slider, the core's clamp, and VehicleCameraControl's own std::min/max --
// and they agree because the protocol says so, not because anyone copied anyone.
//
// Which makes `clamped` unreachable from the only control that calls setZoom. A decoder for it
// would take input nothing can produce, and the sentence it would render -- "the camera went to
// 100 rather than 150" -- describes a press no operator can perform. That is worse than absent:
// it reads as a handled case, so nobody checks whether it is handled.
//
// What DOES reach the operator is the refusal, and that is wired: no camera, no zoom, and the
// camera declining the level are three distinguishable answers where there was one generic line.
// If a camera ever reports its own range, this is where the decoder goes and `clamped` is already
// being sent.
