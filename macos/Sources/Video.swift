import Foundation
import QGCVideoC

final class VideoStore: ObservableObject, Probeable {
    static let probeID = "video"

    @Published private(set) var status = VideoStatus.unavailable

    func refresh() {
        let read = VideoStatus.read(Bridge.group("video"))
        if read != status { status = read }
        pollNative()
    }

    func clear() {
        if status != .unavailable { status = .unavailable }
        pollNative()
    }

    @Published private(set) var nativeFrames = 0
    @Published private(set) var nativeSize = ""
    @Published private(set) var nativeError = ""

    var nativeRunning: Bool { qgc_video_running() }

    func startNative(_ pipeline: String) -> Bool {
        let started = qgc_video_start(pipeline)
        nativeError = started ? "" : String(cString: qgc_video_last_error())
        pollNative()
        return started
    }

    func stopNative() {
        qgc_video_stop()
        pollNative()
    }

    func pollNative() {
        let frames = Int(qgc_video_frames())
        if frames != nativeFrames { nativeFrames = frames }
        let width = Int(qgc_video_width())
        let height = Int(qgc_video_height())
        let size = width > 0 && height > 0 ? "\(width)\u{00D7}\(height)" : ""
        if size != nativeSize { nativeSize = size }
    }

    func probeState() -> [String: Any] {
        ["available": status.available, "gstreamer": status.gstreamer,
         "decoding": status.decoding, "streaming": status.streaming,
         "summary": status.summary, "activeSource": status.activeSource,
         "configured": status.configuredCameras.count,
         "cameras": status.cameras.map { ["title": $0.title, "status": $0.status,
                                          "connecting": $0.connecting] },
         "nativeAvailable": qgc_video_available(), "nativeRunning": nativeRunning,
         "nativeFrames": nativeFrames, "nativeSize": nativeSize, "nativeError": nativeError]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "refresh": refresh()
        case "startNative":
            guard let pipeline = args["pipeline"], !pipeline.isEmpty else {
                return ["ok": false, "error": "startNative needs a pipeline"]
            }
            guard startNative(pipeline) else {
                return ["ok": false, "error": nativeError, "state": probeState()]
            }
        case "stopNative": stopNative()
        case "poll": pollNative()
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
