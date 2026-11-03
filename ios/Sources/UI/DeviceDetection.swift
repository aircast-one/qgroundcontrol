import Foundation

struct DetectModel: Equatable {
    let name: String
    let path: String
    let about: String
    let size: String
    let classes: DeviceBadge
    let engine: String
    let active: Bool
}

struct DetectCamera: Equatable {
    let name: String
    let on: Bool
    let status: DeviceBadge
}

struct DetectionView: Equatable {
    let status: DeviceBadge
    let unavailable: Bool
    let cameras: [DetectCamera]
    let models: [DetectModel]
}

let NO_LABELS = "no labels"

func parseDetection(_ json: JSON) -> DetectionView? {
    guard let status = json["status"].objectOrNil else { return nil }
    return DetectionView(
        status: parseBadge(status),
        unavailable: json["unavailable"].bool,
        cameras: json["cameras"].objects.map { DetectCamera(name: $0["name"].string, on: $0["on"].bool, status: parseBadge($0["status"].objectOrNil)) },
        models: json["models"].objects.map {
            DetectModel(
                name: $0["name"].string,
                path: $0["path"].string,
                about: $0["about"].string,
                size: $0["size"].string,
                classes: parseBadge($0["classes"].objectOrNil),
                engine: $0["engine"].string,
                active: $0["active"].bool
            )
        }
    )
}

func detecting(_ view: DetectionView) -> Bool { view.cameras.contains { $0.on } }

let MODELS_PATH = "/api/detect/models"

func modelPath(_ name: String) -> String { "\(MODELS_PATH)/\(uriComponent(name))" }

struct ModelUpload: Equatable {
    let hef: URL
    let labels: URL?
    let about: String
    let use: Bool
}

func modelFields(_ upload: ModelUpload) -> [DevicePart] {
    var parts = [DevicePart(name: "hef", file: upload.hef)]
    if let labels = upload.labels { parts.append(DevicePart(name: "labels", file: labels)) }
    let about = upload.about.trimmed
    if !about.isEmpty { parts.append(DevicePart(name: "about", value: about)) }
    if upload.use { parts.append(DevicePart(name: "use", value: "1")) }
    return parts
}

struct ModelUploadOutcome: Equatable {
    let refusal: DeviceRefusal?
    let message: String
}

func modelUploadMessage(_ switched: Bool, _ use: Bool, _ cameraDetecting: Bool) -> ModelUploadOutcome {
    if switched && cameraDetecting { return ModelUploadOutcome(refusal: nil, message: "Model uploaded and switched — detection restarts on it.") }
    if switched {
        return ModelUploadOutcome(refusal: nil, message: "Model uploaded and switched — no camera is detecting yet. Turn it on for a camera on this page.")
    }
    if use { return ModelUploadOutcome(refusal: DeviceRefusal(message: "Model uploaded, but switching to it failed — pick it from the list.", field: ""), message: "") }
    return ModelUploadOutcome(refusal: nil, message: "Model uploaded — use it from the list when you want it.")
}

func uploadModel(_ upload: ModelUpload, _ cameraDetecting: Bool) async -> ModelUploadOutcome {
    let answer = await deviceUpload(.POST, MODELS_PATH, modelFields(upload))
    if let refusal = answer.refusal { return ModelUploadOutcome(refusal: refusal, message: "") }
    return modelUploadMessage(answer.result?["switched"].bool ?? false, upload.use, cameraDetecting)
}

func labelsPath(_ name: String) -> String { "\(modelPath(name))/labels" }

func addLabels(_ name: String, _ labels: URL) async -> DeviceRefusal? {
    await deviceUpload(.PUT, labelsPath(name), [DevicePart(name: "labels", file: labels)]).refusal
}
