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
    // The core calls this photoMode; here it is captureMode, because CameraControl.photoMode
    // already means the camera being in photo rather than video. These are different questions --
    // one is which of the two modes the camera is in, the other is what a shutter press does
    // inside photo mode -- and one name for both is the fault this file keeps finding elsewhere.
    let captureMode: String
    let lapseSeconds: Double?
    let lapseCount: Int?
    let lapseUnlimited: Bool
    let canStopPhoto: Bool
    let reportsStorage: Bool

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

    // The core's token for an interval capture, spelled ONCE. CaptureStart compares its own
    // `started` against it too -- a different subject (what one press began, not what mode the
    // camera is in), the same word off the same wire.
    static let timelapse = "timelapse"
    var timelapseMode: Bool { captureMode == CameraControl.timelapse }

    // A shutter that starts an interval capture must not be labelled the same as one that takes a
    // photo: the label is the last thing an operator reads before pressing it, and an interval
    // keeps going after the press. Lives here rather than in FlyWindow, which swift-checks does
    // not compile -- three arms off two of this type's own fields.
    static let shutterBusy = "Taking\u{2026}"
    var shutterTitle: String {
        if isTakingPhoto { return CameraControl.shutterBusy }
        return timelapseMode ? "Start interval" : "Take photo"
    }

    // modeBusy was already here and its partner was spelled at the call site, so half the rule sat
    // where nothing could check it. Both arms now answer from one place.
    static let modeHelp = "Switch between photo and video"
    var modeHint: String { canChangeMode ? CameraControl.modeHelp : CameraControl.modeBusy }

    // pickerShowsCurrentMode was already pinned and the value it gates was spelled at the row, so
    // this is the same half-a-rule shape as modeHint. The empty arm is the ordinary case: when the
    // picker CAN show the mode, repeating it beside the picker says nothing.
    var modeAside: String { pickerShowsCurrentMode ? "" : modeText }

    // A recording camera and an idle one must not carry the same mark. The tint beside it stays at
    // the call site on purpose -- a Color needs SwiftUI and no compiled model imports it -- but it
    // reads the same isRecording this does, so the two cannot drift.
    static let recordingMark = "record.circle.fill"
    static let idleMark = "camera.fill"
    var recordingSymbol: String { isRecording ? CameraControl.recordingMark : CameraControl.idleMark }

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
        captureMode = ""
        lapseSeconds = nil
        lapseCount = nil
        lapseUnlimited = false
        canStopPhoto = false
        reportsStorage = true
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
        captureMode = text("photoMode")
        lapseSeconds = (json["lapseSeconds"] as? NSNumber)?.doubleValue
        lapseCount = (json["lapseCount"] as? NSNumber)?.intValue
        lapseUnlimited = flag("lapseUnlimited")
        canStopPhoto = flag("canStopPhoto")
        reportsStorage = (json["reportsStorage"] as? NSNumber)?.boolValue ?? true
    }

    // One place decides whether each control exists, so the store's guard and the view's
    // condition cannot drift apart. Both are the core's answer, not a re-reading of the mode.
    var offersShutter: Bool { present && canPhoto }
    var offersRecord: Bool { present && canRecord }

    // The record control spelled its label, its symbol and its tint at the call site in
    // FlyWindow, which swift-checks does not compile -- three rules on one flag, and the
    // components row has already shown what happens when a later hand moves one of a set and
    // not the others. The two an operator READS come here and are answered by one call, so the
    // word and the mark cannot disagree about what the camera is doing.
    //
    // The tint stays at the call site deliberately and is the one knowing exception: its
    // CONDITION is isRecording, a served flag the contract already pins, and its value is a
    // single highlight token rather than a severity ladder with arms that could be swapped. No
    // compiled model here imports SwiftUI, and pulling it in for one .tint would be a bigger
    // change than the rule is worth.
    static func recordLabel(_ recording: Bool) -> String { recording ? "Stop" : "Record" }

    static func recordSymbol(_ recording: Bool) -> String {
        recording ? "stop.circle.fill" : "record.circle"
    }
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

// What a shutter press STARTED, read from the action's answer and never from view.camera. The
// camera's configured mode can change between the press and the next poll, so a sentence built
// from the view would eventually report "started 10 shots" for a press that took one. The answer
// describes the command; the view describes the camera now.
struct CaptureStart: Equatable {
    let started: String
    let lapseSeconds: Double?
    let lapseCount: Int?
    let lapseUnlimited: Bool

    init?(_ answer: [String: Any]) {
        guard let started = answer["started"] as? String, !started.isEmpty else { return nil }
        self.started = started
        lapseSeconds = (answer["lapseSeconds"] as? NSNumber)?.doubleValue
        lapseCount = (answer["lapseCount"] as? NSNumber)?.intValue
        lapseUnlimited = (answer["lapseUnlimited"] as? NSNumber)?.boolValue ?? false
    }

    var timelapse: Bool { started == CameraControl.timelapse }

    // A single photo says nothing: the shot counter moves and the operator watched themselves
    // press it. An interval capture has to announce itself, because it keeps going after the press
    // and -- unlimited -- until something stops it. Same press, and only one of the two is news.
    var notice: String {
        guard timelapse else { return "" }
        let every = lapseSeconds.map { " every \(Measure.settled(String(format: "%.0f", $0))) s" } ?? ""
        // lapseCount 0 is MAV_CMD_IMAGE_START_CAPTURE's UNLIMITED, so printing the number says
        // precisely the opposite of what it means: "0 shots" reads as nothing was started.
        guard !lapseUnlimited else { return "Started an interval capture\(every). It will not stop on its own." }
        guard let lapseCount, lapseCount > 0 else { return "Started an interval capture\(every)." }
        return "Started \(lapseCount) shots\(every)."
    }
}

// The storage row is hidden only when the camera has POSITIVELY said it does not track storage.
// QGC's PhotoVideoControl does the same on `storageStatus !== STORAGE_NOT_SUPPORTED`, and the row
// was showing "Not reported" for that camera forever.
//
// The default is true on purpose, in both directions. A camera that has not answered yet keeps its
// row: hiding on silence is the failure this gate was deliberately not built on top of a month --
// an hour -- ago, when absence was served as NOT_SUPPORTED and the row would have vanished for
// every camera in every default build. And an answer with no reportsStorage at all is an older
// core, not a camera making a claim.
extension CameraControl {
    var showsStorage: Bool { reportsStorage }
}
