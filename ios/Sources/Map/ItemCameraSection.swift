import SwiftUI

struct ItemCameraSection: View {
    let index: Int
    var onChanged: () -> Void = {}
    @State private var revision = 0
    @State private var camera: JSON?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let choices = cameraChoices(camera) {
                let picked = choices.labels.indices.contains(choices.chosen) ? choices.labels[choices.chosen] : nil
                if let note = itemCameraTextBeside(camera, picked) {
                    CameraNote(text: note)
                }
                Menu {
                    ForEach(Array(choices.labels.enumerated()), id: \.offset) { at, label in
                        Button(label) { write { ItemCameraBridge.chooseAction(index, at) } }
                    }
                } label: {
                    PlanMenuField(label: "Camera", value: picked ?? "\u{2026}")
                }
                if let extras = cameraExtras(camera) {
                    CameraSectionExtras(extras: extras) { member, value in
                        write { ItemCameraBridge.set(index, member, value) }
                    }
                }
                if let note = itemCameraNote(camera) {
                    CameraNote(text: note)
                }
            }
        }
        .task(id: [index, revision]) {
            let index = index
            camera = await offMain { ItemCameraBridge.read(index) }
        }
    }

    private func write(_ work: @escaping @Sendable () -> Bool) {
        Task { @MainActor in
            let _: Bool = await offMain(work)
            revision += 1
            onChanged()
        }
    }
}

private struct CameraNote: View {
    let text: String
    @Environment(\.theme) private var theme

    var body: some View {
        Text(text)
            .font(.bodySmall)
            .foregroundStyle(theme.colors.onSurfaceVariant)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.vertical, 4)
    }
}
