import Foundation

struct VideoCamera: Identifiable, Equatable {
    let slot: Int
    let title: String
    let status: String
    let connecting: Bool
    let recording: Bool
    let configured: Bool

    var id: Int { slot }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let slot = (json["slot"] as? NSNumber)?.intValue else { return nil }
        self.slot = slot
        title = (json["title"] as? String) ?? ""
        status = (json["status"] as? String) ?? ""
        connecting = (json["connecting"] as? NSNumber)?.boolValue ?? false
        recording = (json["recording"] as? NSNumber)?.boolValue ?? false
        configured = (json["configured"] as? NSNumber)?.boolValue ?? false
    }
}

struct VideoStatus: Equatable {
    let available: Bool
    let gstreamer: Bool
    let streamSource: Bool
    let decoding: Bool
    let streaming: Bool
    let recording: Bool
    let activeSource: Int
    let multipleSources: Bool
    let anyConnecting: Bool
    let configuredCount: Int
    let summary: String
    let cameras: [VideoCamera]

    static let unavailable = VideoStatus()

    private init() {
        available = false
        gstreamer = false
        streamSource = false
        decoding = false
        streaming = false
        recording = false
        activeSource = 0
        multipleSources = false
        anyConnecting = false
        configuredCount = 0
        summary = ""
        cameras = []
    }

    init(_ json: [String: Any]) {
        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }
        available = flag("available")
        gstreamer = flag("gstreamer")
        streamSource = flag("streamSource")
        decoding = flag("decoding")
        streaming = flag("streaming")
        recording = flag("recording")
        activeSource = (json["activeSource"] as? NSNumber)?.intValue ?? 0
        multipleSources = flag("multipleSources")
        anyConnecting = flag("anyConnecting")
        configuredCount = (json["configuredCount"] as? NSNumber)?.intValue ?? 0
        summary = (json["summary"] as? String) ?? ""
        cameras = ((json["cameras"] as? [Any]) ?? []).compactMap(VideoCamera.init)
    }

    var configuredCameras: [VideoCamera] { cameras.filter(\.configured) }

    var settled: Bool { decoding }
}
