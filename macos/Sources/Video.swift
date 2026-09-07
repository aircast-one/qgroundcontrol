import Foundation
import QGCVideoC

final class VideoStore: ObservableObject, Probeable {
    static let probeID = "video"

    static let shared = VideoStore()

    @Published private(set) var status = VideoStatus.unavailable

    @Published private(set) var camera = CameraControl.absent
    @Published private(set) var cameraLabels: [String] = []
    @Published private(set) var sources: [VideoSource] = []
    @Published private(set) var sourceTypes: [String] = []

    private var askedForNative = false

    private static let sourcesPath = "settings.videoSettings.extraVideoSources"

    func loadSources() {
        let fact = Bridge.group(VideoStore.sourcesPath)
        let listed = VideoSources.decode((fact["valueString"] as? String) ?? "")
        if listed != sources { sources = listed }

        let types = ((Bridge.group("settings.videoSettings.videoSource")["enumStrings"] as? [String]) ?? [])
            .filter { !$0.isEmpty }
        if types != sourceTypes { sourceTypes = types }
    }

    func write(_ replacement: VideoSource) {
        let updated = VideoSources.replacing(sources, at: replacement.slot, with: replacement)
        _ = Bridge.set(VideoStore.sourcesPath, VideoSources.encode(updated))
        loadSources()
    }

    func repair(_ source: VideoSource) {
        guard let repaired = VideoSources.repairs(source) else { return }
        write(repaired)
    }

    func useNativeRendering() {
        guard !askedForNative, qgc_video_available() else { return }
        askedForNative = true
        Bridge.invoke("video.setNativeRendering", [true])
        refresh()
    }

    func refresh() {
        let read = VideoStatus.read(Bridge.group("video"))
        if read != status { status = read }
        pollNative()
        loadCamera()
    }

    func loadCamera() {
        let manager = Bridge.group("vehicle.cameraManager")
        let labels = (manager["cameraLabels"] as? [String]) ?? []
        if labels != cameraLabels { cameraLabels = labels }

        let read = CameraControl.read(Bridge.group("vehicle.cameraManager.currentCameraInstance"))
        if read != camera { camera = read }
    }

    func setCameraMode(photo: Bool) {
        guard camera.present, camera.hasModes else { return }
        Bridge.invoke(photo ? "vehicle.cameraManager.currentCameraInstance.setCameraModePhoto"
                            : "vehicle.cameraManager.currentCameraInstance.setCameraModeVideo")
        loadCamera()
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
         "nativeRequested": askedForNative,
         "camera": ["present": camera.present, "title": camera.title, "mode": camera.modeText,
                    "state": camera.stateText, "storage": camera.storageText,
                    "recording": camera.isRecording, "labels": cameraLabels],
         "sources": sources.map { ["slot": $0.slot, "name": $0.name, "source": $0.source,
                                   "url": $0.url, "summary": $0.summary,
                                   "misconfigured": $0.misconfigured] },
         "nativeFrames": nativeFrames, "nativeSize": nativeSize, "nativeError": nativeError]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "refresh":
            refresh()
            loadSources()
        case "setSourceUrl":
            guard let slot = Int(args["slot"] ?? ""),
                  let existing = sources.first(where: { $0.slot == slot }) else {
                return ["ok": false, "error": "no source slot \(args["slot"] ?? "")"]
            }
            var replacement = existing
            replacement.url = args["url"] ?? ""
            write(replacement)
        case "repairSource":
            guard let slot = Int(args["slot"] ?? ""),
                  let existing = sources.first(where: { $0.slot == slot }) else {
                return ["ok": false, "error": "no source slot \(args["slot"] ?? "")"]
            }
            guard VideoSources.repairs(existing) != nil else {
                return ["ok": false, "error": "slot \(slot) has nothing to repair"]
            }
            repair(existing)
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
