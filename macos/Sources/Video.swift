import Foundation

final class VideoStore: ObservableObject, Probeable {
    static let probeID = "video"

    @Published private(set) var status = VideoStatus.unavailable

    func refresh() {
        let read = VideoStatus.read(Bridge.group("video"))
        if read != status { status = read }
    }

    func clear() {
        if status != .unavailable { status = .unavailable }
    }

    func probeState() -> [String: Any] {
        ["available": status.available, "gstreamer": status.gstreamer,
         "decoding": status.decoding, "streaming": status.streaming,
         "summary": status.summary, "activeSource": status.activeSource,
         "configured": status.configuredCameras.count,
         "cameras": status.cameras.map { ["title": $0.title, "status": $0.status,
                                          "connecting": $0.connecting] }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "refresh" else {
            return ["ok": false, "error": "unknown action \(action)"]
        }
        refresh()
        return ["ok": true, "state": probeState()]
    }
}
