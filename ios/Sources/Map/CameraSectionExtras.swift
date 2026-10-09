import SwiftUI

private let CAMERA_MODES = ["Photo", "Video", "Survey"]

private struct NumberEntry: View {
    let label: String
    let value: Double
    var enabled: Bool = true
    let onDone: (Double) -> Void
    @State private var typed = ""

    var body: some View {
        PlanTextField(label: label, text: $typed, enabled: enabled) {
            typedNumber(typed).map(onDone)
        }
        .frame(maxWidth: 110)
        .onChange(of: value, initial: true) { typed = trimmedNumber(value) }
    }
}

func trimmedNumber(_ value: Double) -> String {
    value == value.rounded(.down) && abs(value) < 1e18 ? String(Int64(value)) : String(value)
}

struct CameraSectionExtras: View {
    let extras: CameraExtras
    let write: (String, Any) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            if let time = extras.intervalTime {
                NumberEntry(label: "Time (s)", value: time) { write("cameraPhotoIntervalTime", $0) }
            }
            if let distance = extras.intervalDistance {
                NumberEntry(label: "Distance (\(extras.distanceUnits))", value: distance) { write("cameraPhotoIntervalDistance", $0) }
            }
            if extras.modeSupported {
                HStack(spacing: 6) {
                    Text("Mode").font(.labelMedium).fixedSize()
                    Toggle("Mode", isOn: Binding(get: { extras.commandsMode }, set: { write("specifyCameraMode", $0) }))
                        .labelsHidden()
                    ForEach(Array(CAMERA_MODES.enumerated()), id: \.offset) { at, label in
                        PlanChip(label: label, selected: extras.mode == at, enabled: extras.commandsMode) { write("cameraMode", at) }
                            .fixedSize()
                    }
                }
            }
            HStack(spacing: 6) {
                Text("Gimbal").font(.labelMedium).fixedSize()
                Toggle("Gimbal", isOn: Binding(get: { extras.commandsGimbal }, set: { write("specifyGimbal", $0) }))
                    .labelsHidden()
                NumberEntry(label: "Pitch", value: extras.pitch, enabled: extras.commandsGimbal) { typed in
                    write("gimbalPitch", extras.pitchRange.map { min(max(typed, $0.lowerBound), $0.upperBound) } ?? typed)
                }
                NumberEntry(label: "Yaw", value: extras.yaw, enabled: extras.commandsGimbal) { typed in
                    write("gimbalYaw", extras.yawRange.map { min(max(typed, $0.lowerBound), $0.upperBound) } ?? typed)
                }
            }
            if let range = extras.pitchRange {
                AngleSlider(value: extras.pitch, range: range, enabled: extras.commandsGimbal) { write("gimbalPitch", $0) }
            }
            if let range = extras.yawRange {
                AngleSlider(value: extras.yaw, range: range, enabled: extras.commandsGimbal) { write("gimbalYaw", $0) }
            }
        }
    }
}

private struct AngleSlider: View {
    let value: Double
    let range: ClosedRange<Double>
    let enabled: Bool
    let onDone: (Double) -> Void
    @State private var shown: Double = 0

    var body: some View {
        Slider(value: $shown, in: range) { editing in
            if !editing { onDone((shown + 0.5).rounded(.down)) }
        }
        .disabled(!enabled)
        .onChange(of: [value, range.lowerBound, range.upperBound], initial: true) {
            shown = min(max(value, range.lowerBound), range.upperBound)
        }
    }
}
