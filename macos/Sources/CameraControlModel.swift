import Foundation

struct CameraControl: Equatable {
    var present = false
    var model = ""
    var vendor = ""
    var mode = CameraControl.undefinedMode
    var photoStatus = CameraControl.photoIdle
    var videoStatus = CameraControl.videoStopped
    var recordTime = ""
    var storageStatus = CameraControl.storageUnsupported
    var shots = 0
    var storageFree = ""
    var capturesPhotos = false
    var capturesVideo = false
    var hasModes = false
    var hasZoom = false
    var zoomLevel = 1.0
    var batteryRemaining = -1

    static let absent = CameraControl()

    static let undefinedMode = -1
    static let photoMode = 0
    static let videoMode = 1
    static let surveyMode = 2

    static let videoStopped = 0
    static let videoRunning = 1

    static let photoIdle = 0
    static let photoInProgress = 1
    static let photoIntervalIdle = 2
    static let photoIntervalInProgress = 3

    static let storageEmpty = 0
    static let storageUnformatted = 1
    static let storageReady = 2
    static let storageUnsupported = 3

    static let idleClock = "00:00:00"

    var isRecording: Bool { videoStatus == CameraControl.videoRunning }

    var isTakingPhoto: Bool {
        photoStatus == CameraControl.photoInProgress
            || photoStatus == CameraControl.photoIntervalInProgress
    }

    var title: String {
        if !model.isEmpty { return model }
        return vendor.isEmpty ? "Camera" : vendor
    }

    var modeText: String {
        switch mode {
        case CameraControl.photoMode: return "Photo"
        case CameraControl.videoMode: return "Video"
        case CameraControl.surveyMode: return "Survey"
        default: return "Not set"
        }
    }

    var modeKnown: Bool { mode != CameraControl.undefinedMode }

    var stateText: String {
        if isRecording { return recordTime.isEmpty ? "Recording" : "Recording \(recordTime)" }
        if isTakingPhoto { return "Taking a photo" }
        return "Idle"
    }

    var storageText: String {
        switch storageStatus {
        case CameraControl.storageEmpty: return "No card"
        case CameraControl.storageUnformatted: return "Not formatted"
        case CameraControl.storageReady: return storageFree.isEmpty ? "Ready" : storageFree
        default: return "Not reported"
        }
    }

    var clockText: String { isRecording ? (recordTime.isEmpty ? CameraControl.idleClock : recordTime)
                                        : CameraControl.idleClock }

    var shotsText: String { String(format: "%05d", shots) }

    var batteryText: String {
        batteryRemaining >= 0 ? "\(batteryRemaining)%" : ""
    }

    var canRecord: Bool { capturesVideo && (!hasModes || mode != CameraControl.photoMode) }

    var canPhoto: Bool { capturesPhotos && (!hasModes || mode != CameraControl.videoMode) }

    static func read(_ json: [String: Any]) -> CameraControl {
        guard json["kind"] as? String == "object",
              let model = json["modelName"] as? String, !model.isEmpty else { return .absent }

        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }
        func text(_ name: String) -> String { (json[name] as? String) ?? "" }
        func number(_ name: String, _ fallback: Int) -> Int {
            (json[name] as? NSNumber)?.intValue ?? fallback
        }

        var camera = CameraControl()
        camera.present = true
        camera.model = model
        camera.vendor = text("vendor")
        camera.mode = number("cameraMode", CameraControl.undefinedMode)
        camera.photoStatus = number("photoCaptureStatus", CameraControl.photoIdle)
        camera.videoStatus = number("videoCaptureStatus", CameraControl.videoStopped)
        camera.recordTime = text("recordTimeStr")
        camera.storageStatus = number("storageStatus", CameraControl.storageUnsupported)
        camera.storageFree = text("storageFreeStr")
        camera.capturesPhotos = flag("capturesPhotos")
        camera.capturesVideo = flag("capturesVideo")
        camera.hasModes = flag("hasModes")
        camera.hasZoom = flag("hasZoom")
        camera.zoomLevel = (json["zoomLevel"] as? NSNumber)?.doubleValue ?? 1
        camera.batteryRemaining = (json["batteryRemaining"] as? NSNumber)?.intValue ?? -1
        return camera
    }
}
