import Foundation
import QGCVideoC

final class VideoStore: ObservableObject, Probeable, WriteReporting {
    @Published var writeFailure: String?
    static let probeID = "video"

    static let shared = VideoStore()

    @Published private(set) var status = VideoStatus.unavailable

    @Published private(set) var camera = CameraControl.absent
    @Published private(set) var cameraLabels: [String] = []
    @Published private(set) var sources: [VideoSource] = []

    private var askedForNative = false

    private static let sourcesPath = "settings.videoSettings.extraVideoSources"

    func loadSources() {
        let fact = Bridge.group(VideoStore.sourcesPath)
        let cameras = ((Bridge.group("view.video")["cameras"] as? [Any]) ?? [])
            .compactMap(VideoCamera.init)
        let listed = VideoSources.decode((fact["valueString"] as? String) ?? "", cameras: cameras)
        if listed != sources { sources = listed }
    }

    func write(_ replacement: VideoSource) {
        let updated = VideoSources.replacing(sources, at: replacement.slot, with: replacement)
        write(VideoStore.sourcesPath, VideoSources.encode(updated), "the video source")
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

    private var watchPoll: Timer?

    // view.video is built entirely from video.* -- the manager and the settings, not one vehicle
    // path -- so a source configured while no vehicle is talking must still reach the Fly view.
    func startWatching() {
        guard watchPoll == nil else { return }
        refresh()
        watchPoll = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            self?.refresh()
        }
    }

    func stopWatching() {
        watchPoll?.invalidate()
        watchPoll = nil
    }

    private(set) var refreshes = 0

    func refresh() {
        refreshes += 1
        let read = VideoStatus(Bridge.group("view.video"))
        if read != status { status = read }
        pollNative()
        loadCamera()
    }

    func loadCamera() {
        let manager = Bridge.group("vehicle.cameraManager")
        let labels = (manager["cameraLabels"] as? [String]) ?? []
        if labels != cameraLabels { cameraLabels = labels }

        let read = CameraControl(Bridge.group("view.camera"))
        if read != camera { camera = read }
    }

    func setCameraMode(photo: Bool) {
        guard camera.present, camera.hasModes else { return }
        Bridge.invoke(photo ? "vehicle.cameraManager.currentCameraInstance.setCameraModePhoto"
                            : "vehicle.cameraManager.currentCameraInstance.setCameraModeVideo")
        loadCamera()
    }

    // The gates are the core's canPhoto and canRecord, not this head's reading of the mode.
    // QGC additionally refuses while a capture is already running; that term is not in the
    // core's answer, so it is not invented here either.
    func takePhoto() {
        guard camera.offersShutter else { return }
        Bridge.invoke("vehicle.cameraManager.currentCameraInstance.takePhoto")
        loadCamera()
    }

    func toggleRecording() {
        guard camera.offersRecord else { return }
        Bridge.invoke("vehicle.cameraManager.currentCameraInstance.toggleVideoRecording")
        loadCamera()
    }

    // The C++ owns the rotation order and persists the choice; the head only asks for the next one.
    func switchSource() {
        guard status.offersSwitch else { return }
        Bridge.invoke("video.switchActiveVideoSource")
        refresh()
    }

    func setZoom(_ level: Double) {
        guard camera.present, camera.hasZoom, level.isFinite else { return }
        write("vehicle.cameraManager.currentCameraInstance.zoomLevel", level,
              "the camera zoom")
        loadCamera()
    }

    func clear() {
        if status != .unavailable { status = .unavailable }
        stopDetections()
        pollNative()
    }

    @Published private(set) var detections = Detections.none

    private var detectionPoll: Timer?

    // The boxes come from the camera host, not the vehicle, so they cannot ride the telemetry
    // change that drives refresh(): a silent vehicle would freeze them on screen.
    func startDetections() {
        guard detectionPoll == nil else { return }
        refreshDetections()
        detectionPoll = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { [weak self] _ in
            self?.refreshDetections()
        }
    }

    func stopDetections() {
        detectionPoll?.invalidate()
        detectionPoll = nil
        if detections != .none { detections = .none }
    }

    func refreshDetections() {
        let read = Detections(Bridge.group("view.detections"))
        if read != detections { detections = read }
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
        ["writeFailure": writeFailure ?? "",
         "available": status.available, "gstreamer": status.gstreamer,
         "decoding": status.decoding, "streaming": status.streaming,
         "summary": status.summary, "activeSource": status.activeSource,
         "multipleSources": status.multipleSources, "offersSwitch": status.offersSwitch,
         "activeCameraTitle": status.activeCameraTitle,
         "configured": status.configuredCameras.count,
         "cameras": status.cameras.map { ["title": $0.title, "status": $0.status,
                                          "connecting": $0.connecting] },
         "refreshes": refreshes, "watching": watchPoll != nil,
         "sourceSize": status.sourceSize.map { ["width": $0.width, "height": $0.height] } ?? [:],
         "detections": ["available": detections.available, "stale": detections.stale,
                        "draws": detections.draws, "error": detections.error,
                        "polling": detectionPoll != nil,
                        "boxes": detections.boxes.map {
                            ["caption": $0.caption, "x": $0.x, "y": $0.y,
                             "w": $0.width, "h": $0.height]
                        }],
         "nativeAvailable": qgc_video_available(), "nativeRunning": nativeRunning,
         "nativeRequested": askedForNative,
         "camera": ["present": camera.present, "title": camera.title, "mode": camera.modeText,
                    "state": camera.stateText, "storage": camera.storageText,
                     "shots": camera.shotsText, "clock": camera.clockText,
                     "battery": camera.batteryText, "hasZoom": camera.hasZoom,
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
