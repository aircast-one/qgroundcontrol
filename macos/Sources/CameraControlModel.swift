import Foundation

struct CameraControl: Equatable {
    var present = false
    var model = ""
    var vendor = ""
    var mode = ""
    var photoStatus = ""
    var videoStatus = ""
    var recordTime = ""
    var storageStatus = ""
    var storageFree = ""
    var capturesPhotos = false
    var capturesVideo = false
    var hasModes = false
    var hasZoom = false
    var zoomLevel = 1.0
    var batteryRemaining = -1

    static let absent = CameraControl()

    static let undefinedMode = "CAM_MODE_UNDEFINED"
    static let photoMode = "CAM_MODE_PHOTO"
    static let videoMode = "CAM_MODE_VIDEO"
    static let surveyMode = "CAM_MODE_SURVEY"
    static let recording = "VIDEO_CAPTURE_STATUS_RUNNING"
    static let takingPhoto = "PHOTO_CAPTURE_IN_PROGRESS"
    static let storageUnsupported = "STORAGE_NOT_SUPPORTED"

    var isRecording: Bool { videoStatus == CameraControl.recording }

    var isTakingPhoto: Bool { photoStatus == CameraControl.takingPhoto }

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

    var modeKnown: Bool { mode != CameraControl.undefinedMode && !mode.isEmpty }

    var stateText: String {
        if isRecording { return recordTime.isEmpty ? "Recording" : "Recording \(recordTime)" }
        if isTakingPhoto { return "Taking a photo" }
        return "Idle"
    }

    var storageText: String {
        if storageStatus == CameraControl.storageUnsupported { return "Not reported" }
        return storageFree.isEmpty ? "Unknown" : storageFree
    }

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

        var camera = CameraControl()
        camera.present = true
        camera.model = model
        camera.vendor = text("vendor")
        camera.mode = text("cameraMode")
        camera.photoStatus = text("photoCaptureStatus")
        camera.videoStatus = text("videoCaptureStatus")
        camera.recordTime = text("recordTimeStr")
        camera.storageStatus = text("storageStatus")
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
