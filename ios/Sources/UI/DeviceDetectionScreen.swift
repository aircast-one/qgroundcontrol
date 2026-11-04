import SwiftUI

private let MODEL_FIELD = "hef"

struct DeviceDetectionScreen: View {
    let props: DeviceScreenProps
    @DeviceView("detection") private var reading

    var body: some View {
        NativeForm {
            if let view = deviceView(reading, parseDetection) {
                DeviceSection(title: "Running") {
                    DeviceFact(label: "In use", value: view.status.text)
                    if view.unavailable {
                        DeviceNotice(
                            notice: DeviceBadge(
                                level: DeviceLevel.Warn,
                                text: "This device has no AI accelerator, so .hef models can't run. Switch to an .onnx model to detect on the CPU. Draw-a-box tracking still works on every camera."
                            )
                        )
                    }
                }
                DetectionCameras(cameras: view.cameras, onOpen: props.onOpen)
                ModelsSection(view: view)
                FootNote(
                    text: "Detection runs on the device and sends boxes as data — the picture itself is untouched. Pick the model for what the camera is looking at: an aerial model hallucinates trucks on a desk. " +
                        "Draw-a-box tracking works whichever model is active."
                )
            } else {
                FootNote(text: DEVICE_WAITING)
            }
        }
    }
}

private struct DetectionCameras: View {
    let cameras: [DetectCamera]
    let onOpen: (String) -> Void

    var body: some View {
        DeviceSection(title: "Cameras") {
            if cameras.isEmpty {
                DeviceText(text: "No cameras yet.")
                SetupRow(title: "Cameras", onClick: { onOpen("/streaming") })
            } else {
                ForEach(cameras, id: \.name) { camera in
                    DeviceSwitchRow(
                        title: "Draw boxes on \(camera.name)",
                        checked: camera.on,
                        on: DeviceRequest(method: .PUT, path: "/api/streams/\(camera.name)/detect"),
                        off: DeviceRequest(method: .DELETE, path: "/api/streams/\(camera.name)/detect"),
                        detail: camera.status.text,
                        success: "Detection turned on for \(camera.name).",
                        offSuccess: "Detection turned off for \(camera.name)."
                    )
                }
            }
        }
    }
}

private struct ModelsSection: View {
    let view: DetectionView
    @State private var uploads = 0

    var body: some View {
        DeviceSection(title: "Models") {
            if view.models.isEmpty {
                DeviceText(text: "No models installed yet.")
            } else {
                ForEach(view.models, id: \.name) { ModelRow(model: $0) }
            }
            DeviceGroup("Add a model", summary: "A .hef for the Hailo-8 or an .onnx for the CPU.") {
                ModelUploadForm(cameraDetecting: detecting(view)) { uploads += 1 }
            }
            .id(uploads)
        }
    }
}

private struct ModelRow: View {
    let model: DetectModel

    var body: some View {
        HStack(spacing: Space.s3) {
            VStack(alignment: .leading, spacing: 2) {
                Text(model.name)
                if !model.about.isEmpty {
                    Text(model.about).font(.subheadline).foregroundStyle(Color.secondary)
                }
                HStack(spacing: Space.s3) {
                    Text(model.size).font(.footnote).foregroundStyle(Color.secondary)
                    DeviceBadgeText(badge: model.classes)
                    if !model.engine.isEmpty {
                        Text(model.engine).font(.footnote).foregroundStyle(Color.secondary)
                    }
                }
                if model.classes.text == NO_LABELS { AddLabels(name: model.name) }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if model.active {
                DeviceBadgeText(badge: DeviceBadge(level: DeviceLevel.Ready, text: "Active"))
            } else {
                HStack(spacing: Space.s1) {
                    ActionButton(
                        label: "Use",
                        request: DeviceRequest(method: .PUT, path: "/api/stream/detect/settings", body: deviceBody(["hef": model.path])),
                        success: "Model switched — detection restarts on the new model."
                    )
                    DeviceDestructiveButton(
                        label: "Delete",
                        request: DeviceRequest(method: .DELETE, path: modelPath(model.name)),
                        success: "Model deleted.",
                        confirm: ActionConfirm(title: "Delete \(model.name)?", text: "Delete \(model.name)? Anyone relying on it loses it from this device.", confirm: "Delete")
                    )
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(model.name)
    }
}

private struct AddLabels: View {
    let name: String
    @Environment(\.LocalChangeNotice) private var notice
    @State private var busy = false
    @State private var refusal: DeviceRefusal?
    @State private var picking = false

    private func add(_ file: URL?) {
        guard let file else { return }
        busy = true
        let name = name, notice = notice
        Task { @MainActor in
            let refused = await addLabels(name, file)
            busy = false
            refusal = refused
            if refused == nil { notice.show("Labels added to \(name).", nil) }
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Button(busy ? "Adding labels…" : "Add labels") { picking = true }
                .buttonStyle(.text)
                .disabled(busy)
                .accessibilityLabel("Labels for \(name)")
            DeviceRefusalText(refusal: refusal)
        }
        .deviceFilePicker($picking, onPick: add)
    }
}

private struct FilePick: View {
    let label: String
    let description: String
    let file: URL?
    let onPick: (URL?) -> Void
    @State private var picking = false

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            Text(label).font(.subheadline).foregroundStyle(Color.secondary)
            Button(file?.lastPathComponent ?? "Choose a file") { picking = true }
                .buttonStyle(.bordered)
                .accessibilityLabel(description)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .deviceFilePicker($picking, onPick: onPick)
    }
}

private struct ModelUploadForm: View {
    let cameraDetecting: Bool
    let onUploaded: () -> Void
    @Environment(\.LocalChangeNotice) private var notice
    @State private var hef: URL?
    @State private var labels: URL?
    @State private var about = ""
    @State private var use = true
    @State private var busy = false
    @State private var refusal: DeviceRefusal?

    private func upload() {
        guard let hef else {
            refusal = DeviceRefusal(message: "Choose a .hef or .onnx model file.", field: MODEL_FIELD)
            return
        }
        busy = true
        refusal = nil
        let request = ModelUpload(hef: hef, labels: labels, about: about, use: use)
        let cameraDetecting = cameraDetecting, notice = notice, onUploaded = onUploaded
        Task { @MainActor in
            let outcome = await uploadModel(request, cameraDetecting)
            busy = false
            refusal = outcome.refusal
            if !outcome.message.isEmpty { notice.show(outcome.message, nil) }
            if outcome.refusal == nil { onUploaded() }
        }
    }

    var body: some View {
        // Rows inside the Add a model group: the two files, the description, the switch and Upload.
        VStack(alignment: .leading, spacing: Space.s1) {
            FilePick(label: "Model (.hef for the Hailo-8, .onnx for the CPU)", description: "Model file", file: hef) {
                hef = $0
                refusal = nil
            }
            DeviceHint(text: "Trained with Ultralytics? Export it first: yolo export model=best.pt format=onnx — the .onnx carries its class names.")
            DeviceRefusalText(refusal: refusal, field: MODEL_FIELD)
        }
        FilePick(
            label: "Class labels (.labels, one per line — optional). An Ultralytics .onnx brings its own; without labels, boxes are named by class number.",
            description: "Class labels file",
            file: labels
        ) { labels = $0 }
        DeviceTextField(
            label: "What it's for (shown next to the model — optional)",
            value: about,
            onValueChange: { about = $0 },
            placeholder: "Aerial — vehicles at 400 ft"
        )
        Toggle("Switch to this model after upload", isOn: $use)
        DeviceRefusalText(refusal: refusal)
        Button(busy ? "Uploading…" : "Upload model", action: upload).buttonStyle(.borderedProminent).disabled(busy)
    }
}
