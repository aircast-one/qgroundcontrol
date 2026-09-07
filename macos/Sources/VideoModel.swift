import Foundation

struct VideoCamera: Identifiable, Equatable {
    let slot: Int
    let status: String
    let connecting: Bool
    let recording: Bool

    var id: Int { slot }

    var title: String { "Camera \(slot + 1)" }

    var configured: Bool { !status.isEmpty && status != VideoStatus.noUrlStatus }
}

struct VideoStatus: Equatable {
    var available = false
    var gstreamer = false
    var streamSource = false
    var decoding = false
    var streaming = false
    var recording = false
    var activeSource = 0
    var cameras: [VideoCamera] = []

    static let unavailable = VideoStatus()
    static let noUrlStatus = "No stream URL"

    var configuredCameras: [VideoCamera] { cameras.filter(\.configured) }

    var anyConnecting: Bool { cameras.contains(where: \.connecting) }

    var summary: String {
        guard available else { return "This build cannot show video." }
        if decoding { return recording ? "Streaming and recording." : "Streaming." }
        if anyConnecting { return "Waiting for a stream." }
        if configuredCameras.isEmpty { return "No stream URL is set." }
        return "Not streaming."
    }

    var settled: Bool { decoding }

    static func cameras(statuses: [String], connecting: [Bool], recording: [Bool]) -> [VideoCamera] {
        statuses.enumerated().map { slot, status in
            VideoCamera(slot: slot,
                        status: status,
                        connecting: slot < connecting.count ? connecting[slot] : false,
                        recording: slot < recording.count ? recording[slot] : false)
        }
    }

    static func read(_ json: [String: Any]) -> VideoStatus {
        guard json["kind"] as? String == "object" else { return .unavailable }

        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }
        func flags(_ name: String) -> [Bool] {
            ((json[name] as? [Any]) ?? []).map { ($0 as? NSNumber)?.boolValue ?? false }
        }

        var status = VideoStatus()
        status.available = flag("hasVideo")
        status.gstreamer = flag("gstreamerEnabled")
        status.streamSource = flag("isStreamSource")
        status.decoding = flag("decoding")
        status.streaming = flag("streaming")
        status.recording = flag("recording")
        status.activeSource = (json["activeVideoSource"] as? NSNumber)?.intValue ?? 0
        status.cameras = cameras(statuses: (json["cameraStatuses"] as? [String]) ?? [],
                                 connecting: flags("cameraConnecting"),
                                 recording: flags("cameraRecording"))
        return status
    }
}
