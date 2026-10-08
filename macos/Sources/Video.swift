import Foundation
import QGCVideoC

let mainVideoChannel: Int32 = 0
let pipVideoChannel: Int32 = 1

final class VideoStore: ObservableObject, Probeable, WriteReporting {
    @Published var writeFailure: String?
    static let probeID = "video"

    static let shared = VideoStore()

    @Published private(set) var status = VideoStatus.unavailable

    @Published private(set) var camera = CameraControl.absent
    @Published private(set) var cameraLabels: [String] = []
    @Published private(set) var cameraList = CameraList.empty

    private var askedForNative = false

    func loadSources() {
        let read = CameraList(Bridge.group("view.cameras"))
        if read != cameraList { cameraList = read }
    }

    @discardableResult
    func addCamera(name: String, source: String, url: String) -> Bool {
        askCameras("cameras.add", [name, source, url])
    }

    func updateCamera(_ camera: VideoSource, url: String) {
        guard let slot = camera.stored else { return }
        askCameras("cameras.update", [slot, camera.name, camera.source, url])
    }

    func removeCamera(_ camera: VideoSource) {
        guard let slot = camera.stored else { return }
        askCameras("cameras.remove", [slot])
    }

    func classifyCamera(_ address: String) -> CameraGuess {
        CameraGuess(Bridge.invoke("cameras.classify", [address]))
    }

    func showCamera(_ camera: VideoSource) {
        askCameras("video.setActiveVideoSource", [camera.slot])
        refresh()
    }

    func setPip(_ on: Bool) {
        write("settings.videoSettings.multiViewEnabled", on, "picture in picture")
        refresh()
    }

    @discardableResult
    private func askCameras(_ action: String, _ args: [Any]) -> Bool {
        writeFailure = CameraList.refusal(Bridge.invoke(action, args))
        loadSources()
        return writeFailure == nil
    }

    func useNativeRendering() {
        guard !askedForNative, qgc_video_available() else { return }
        askedForNative = true
        Bridge.invoke("video.setNativeRendering", [true])
        Bridge.invoke("video.initNative")
        refresh()
    }

    private var watchPoll: Timer?

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
        drive(read.nativePipeline)
        drivePip(read.pipPipeline)
        record(read.nativeRecording)
        pollNative()
        pollPip()
        loadCamera()
        loadSources()
    }

    func loadCamera() {
        let read = CameraControl(Bridge.group("view.camera"))
        if read != camera { camera = read }
        if read.labels != cameraLabels { cameraLabels = read.labels }
    }

    func setCameraMode(photo: Bool) {
        guard camera.canChangeMode else { return }
        ask("camera.setMode", [photo ? "photo" : "video"])
    }

    @Published private(set) var captureNotice = ""

    func takePhoto() {
        guard camera.offersShutter else { return }
        let answer = ask("camera.takePhoto", [])
        captureNotice = CaptureStart(answer)?.notice ?? ""
    }

    func stopPhoto() {
        guard camera.canStopPhoto else { return }
        ask("camera.stopPhoto", [])
        captureNotice = ""
    }

    func toggleRecording() {
        guard camera.offersRecord else { return }
        ask("camera.toggleRecording", [])
    }

    @discardableResult
    private func ask(_ action: String, _ args: [Any]) -> [String: Any] {
        let answer = Bridge.invoke(action, args)
        writeFailure = CameraRefusal.sentence(answer)
        loadCamera()
        return answer
    }

    func switchSource() {
        guard status.offersSwitch else { return }
        Bridge.invoke("video.switchActiveVideoSource")
        refresh()
    }

    func setZoom(_ level: Double) {
        guard camera.present, camera.hasZoom, level.isFinite else { return }
        write("vehicle.cameraManager.currentCameraInstance.zoomLevel", level, "the camera zoom")
        loadCamera()
    }

    func clear() {
        if status != .unavailable { status = .unavailable }
        stopDetections()
        pollNative()
        pollPip()
    }

    @Published private(set) var detections = Detections.none

    private var detectionPoll: Timer?

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

    var nativeRunning: Bool { qgc_video_running(mainVideoChannel) }

    func startNative(_ pipeline: String) -> Bool {
        let started = qgc_video_start(mainVideoChannel, pipeline)
        nativeError = started ? "" : String(cString: qgc_video_last_error(mainVideoChannel))
        pollNative()
        return started
    }

    func stopNative() {
        qgc_video_stop(mainVideoChannel)
        pollNative()
    }

    private var drivenRecording: NativeRecording?
    private var reportedRecording = false

    private func record(_ wanted: NativeRecording?) {
        if wanted != drivenRecording {
            if drivenRecording != nil {
                qgc_video_stop_recording(mainVideoChannel)
            }
            if let wanted, !qgc_video_start_recording(mainVideoChannel, wanted.file, wanted.format) {
                nativeError = String(cString: qgc_video_last_error(mainVideoChannel))
            }
            drivenRecording = wanted
        }
        let recording = qgc_video_recording(mainVideoChannel)
        guard recording != reportedRecording else { return }
        reportedRecording = recording
        Bridge.invoke("video.reportRecording", [recording])
    }

    private var drivenPipeline: String?
    private var reported: [Int] = []

    private func drive(_ pipeline: String?) {
        guard pipeline != drivenPipeline else { return }
        let wasDriving = drivenPipeline != nil
        drivenPipeline = pipeline
        if let pipeline, qgc_video_available() {
            _ = startNative(pipeline)
        } else if wasDriving {
            stopNative()
        }
    }

    func pollNative() {
        let frames = Int(qgc_video_frames(mainVideoChannel))
        if frames != nativeFrames { nativeFrames = frames }
        let width = Int(qgc_video_width(mainVideoChannel))
        let height = Int(qgc_video_height(mainVideoChannel))
        let size = width > 0 && height > 0 ? "\(width)\u{00D7}\(height)" : ""
        if size != nativeSize { nativeSize = size }
        guard drivenPipeline != nil else { return }
        let state = [nativeRunning ? 1 : 0, min(frames, 1), width, height]
        guard state != reported else { return }
        reported = state
        Bridge.invoke("video.reportNative", [nativeRunning, frames, width, height, nativeError])
    }

    @Published private(set) var pipFrames = 0
    private var drivenPip: String?
    private var reportedPip: [Int] = []

    private func drivePip(_ pipeline: String?) {
        guard pipeline != drivenPip else { return }
        let wasDriving = drivenPip != nil
        drivenPip = pipeline
        reportedPip = []
        if let pipeline, qgc_video_available() {
            _ = qgc_video_start(pipVideoChannel, pipeline)
        } else if wasDriving {
            qgc_video_stop(pipVideoChannel)
        }
    }

    private func pollPip() {
        let frames = Int(qgc_video_frames(pipVideoChannel))
        if frames != pipFrames { pipFrames = frames }
        guard drivenPip != nil else { return }
        let running = qgc_video_running(pipVideoChannel)
        let width = Int(qgc_video_width(pipVideoChannel))
        let height = Int(qgc_video_height(pipVideoChannel))
        let state = [running ? 1 : 0, min(frames, 1), width, height]
        guard state != reportedPip else { return }
        reportedPip = state
        Bridge.invoke("video.reportNative", [running, frames, width, height,
                                             String(cString: qgc_video_last_error(pipVideoChannel)),
                                             NSNull(), false, Int(pipVideoChannel)])
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
                    "recording": camera.isRecording, "labels": cameraLabels,
                    "captureMode": camera.captureMode, "canStopPhoto": camera.canStopPhoto,
                    "lapseCount": Int(camera.lapseCount ?? -1),
                    "lapseSeconds": Double(camera.lapseSeconds ?? -1),
                    "lapseUnlimited": camera.lapseUnlimited,
                    "captureNotice": captureNotice,
                    "reportsStorage": camera.reportsStorage],
         "sources": cameraList.entries.map { ["slot": $0.slot, "stored": $0.stored ?? -1, "name": $0.name,
                                              "source": $0.source, "url": $0.url, "summary": $0.summary,
                                              "problem": $0.problem ?? "", "fromDrone": $0.fromDrone,
                                              "short": $0.short, "status": $0.status.rawValue] },
         "nativeFrames": nativeFrames, "nativeSize": nativeSize, "nativeError": nativeError,
         "pip": ["offered": cameraList.offersPip, "enabled": cameraList.pipEnabled,
                 "slot": cameraList.pipSlot ?? -1, "driven": drivenPip != nil, "frames": pipFrames]]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "refresh":
            refresh()
        case "setSourceUrl":
            guard let slot = Int(args["slot"] ?? ""),
                  let existing = cameraList.entries.first(where: { $0.stored == slot }) else {
                return ["ok": false, "error": "no stored camera \(args["slot"] ?? "")"]
            }
            updateCamera(existing, url: args["url"] ?? "")
        case "addCamera":
            guard addCamera(name: args["name"] ?? "", source: args["source"] ?? "", url: args["url"] ?? "") else {
                return ["ok": false, "error": writeFailure ?? "", "state": probeState()]
            }
        case "removeCamera":
            guard let slot = Int(args["slot"] ?? ""),
                  let existing = cameraList.entries.first(where: { $0.stored == slot }) else {
                return ["ok": false, "error": "no stored camera \(args["slot"] ?? "")"]
            }
            removeCamera(existing)
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
