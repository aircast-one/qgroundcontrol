import Foundation

struct SourceSize: Equatable {
    let width: Double
    let height: Double

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let width = (json["width"] as? NSNumber)?.doubleValue,
              let height = (json["height"] as? NSNumber)?.doubleValue,
              width.isFinite, height.isFinite, width > 0, height > 0 else { return nil }
        self.width = width
        self.height = height
    }

    // VideoLayerView paints the frame with .resizeAspect, so a source whose ratio differs from
    // the pane is letterboxed and anything normalised to the pane lands in the bars instead.
    func painted(inWidth paneWidth: Double, height paneHeight: Double) -> PaintedPicture? {
        guard paneWidth > 0, paneHeight > 0 else { return nil }
        let scale = min(paneWidth / width, paneHeight / height)
        let shown = (width: width * scale, height: height * scale)
        return PaintedPicture(x: (paneWidth - shown.width) / 2,
                              y: (paneHeight - shown.height) / 2,
                              width: shown.width, height: shown.height)
    }
}

struct PaintedPicture: Equatable {
    let x: Double
    let y: Double
    let width: Double
    let height: Double

    func place(_ box: DetectionBox) -> PaintedPicture {
        PaintedPicture(x: x + box.x * width, y: y + box.y * height,
                       width: box.width * width, height: box.height * height)
    }
}

struct VideoCamera: Identifiable, Equatable {
    let slot: Int
    let title: String
    let status: String
    let connecting: Bool
    let recording: Bool
    let enabled: Bool
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
        enabled = (json["enabled"] as? NSNumber)?.boolValue ?? false
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
    let sourceSize: SourceSize?
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
        sourceSize = nil
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
        sourceSize = SourceSize(json["sourceSize"])
        activeSource = (json["activeSource"] as? NSNumber)?.intValue ?? 0
        multipleSources = flag("multipleSources")
        anyConnecting = flag("anyConnecting")
        configuredCount = (json["configuredCount"] as? NSNumber)?.intValue ?? 0
        summary = (json["summary"] as? String) ?? ""
        cameras = ((json["cameras"] as? [Any]) ?? []).compactMap(VideoCamera.init)
    }

    var configuredCameras: [VideoCamera] { cameras.filter(\.configured) }

    var listedCameras: [VideoCamera] { cameras.filter { $0.configured || $0.enabled } }

    var activeCamera: VideoCamera? { cameras.first { $0.slot == activeSource } }

    // QGC labels the switch with cameraName(activeVideoSource), which falls back to "Camera N".
    // The core already applies that fallback when it builds each title, so a slot with no camera
    // at all means the head has no name to show and does not offer the switch.
    var offersSwitch: Bool { multipleSources && activeCamera != nil }

    var activeCameraTitle: String { activeCamera?.title ?? "" }

    var settled: Bool { decoding }
}
