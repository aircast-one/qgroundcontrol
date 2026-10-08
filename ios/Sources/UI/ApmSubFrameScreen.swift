import SwiftUI
import UIKit

let APM_SUB_FRAME_SCREEN = "apmSubFrame"
let APM_SUB_FRAME_VIEW = "view.apmSubFrame"
let APM_SUB_FRAME_SET = "apmSubFrame.set"
let APM_SUB_FRAME_LOAD = "apmSubFrame.loadDefaults"
private let APM_SUB_FRAME_POLL_MS = 1000

struct SubFrame: Equatable {
    let name: String
    let value: Int
    let hasDefaults: Bool
}

struct SubFrames: Equatable {
    let frames: [SubFrame]
    let selected: Int?
    let confirmFirst: Bool
    let loading: Bool
    let loadError: String
}

func subFrames(_ view: JSON?) -> SubFrames? {
    guard let it = view, it["available"].bool else { return nil }
    return SubFrames(
        frames: it["frames"].array.filter { $0.object != nil }.map { SubFrame(name: $0["name"].string, value: $0["value"].int(0), hasDefaults: $0["hasDefaults"].bool) },
        selected: it["selected"].isNull ? nil : it["selected"].int(0),
        confirmFirst: it["confirmFirst"].bool,
        loading: it["loadingDefaults"].bool,
        loadError: it["loadError"].string
    )
}

func frameImage(_ value: Int) -> UIImage? {
    let png = Qgc.get("view.apmSubFrameImage(\(value))")["png"].string
    guard !png.isBlank, let bytes = Data(base64Encoded: png, options: .ignoreUnknownCharacters) else { return nil }
    return UIImage(data: bytes)
}

struct ApmSubFrameScreen: View {
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var read: SubFrames?
    @State private var picked: SubFrame?
    @State private var refusal: String?

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear
            if let state = read {
                screen(state)
            } else {
                Text("This page is for an ArduSub vehicle.").padding(16)
            }
        }
        .task(id: revision) {
            let frames = await offMain { subFrames(Qgc.get(APM_SUB_FRAME_VIEW)) }
            read = frames
            try? await Task.sleep(for: .milliseconds(APM_SUB_FRAME_POLL_MS))
            if !Task.isCancelled { revision += 1 }
        }
        .alert("Frame selection", isPresented: Binding(get: { picked != nil }, set: { if !$0 { picked = nil } }), presenting: picked) { frame in
            if frame.hasDefaults {
                Button("Yes, Load default parameter set for \(frame.name)") {
                    picked = nil
                    act(APM_SUB_FRAME_LOAD, frame)
                }
            }
            Button(frame.hasDefaults ? "No, set frame only" : "Confirm frame \(frame.name)") {
                picked = nil
                act(APM_SUB_FRAME_SET, frame)
            }
            Button("Close", role: .cancel) { picked = nil }
        } message: { frame in
            Text(frame.hasDefaults ? "Would you like to load the default parameters for the frame?" : "Would you like to set the desired frame?")
        }
    }

    private func screen(_ state: SubFrames) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                if state.loading {
                    Text("Loading the frame's default parameters…").font(.bodySmall)
                }
                if let message = refusal ?? (state.loadError.isBlank ? nil : state.loadError) {
                    Text(message).foregroundStyle(theme.colors.error)
                }
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 160), spacing: 8)], spacing: 8) {
                    ForEach(state.frames, id: \.value) { frame in
                        SubFrameTile(frame: frame, chosen: frame.value == state.selected) {
                            if state.confirmFirst { picked = frame } else { act(APM_SUB_FRAME_SET, frame) }
                        }
                    }
                }
            }
            .padding(12)
        }
    }

    private func act(_ path: String, _ frame: SubFrame) {
        let value = frame.value
        Task { refusal = await offMain { Qgc.refusalOf(path, value) } }
    }
}

private struct SubFrameTile: View {
    let frame: SubFrame
    let chosen: Bool
    let onPick: () -> Void
    @Environment(\.theme) private var theme
    @State private var image: UIImage?

    var body: some View {
        Button(action: onPick) {
            VStack(alignment: .leading, spacing: 4) {
                Text(frame.name).font(.titleSmall)
                if let image {
                    Color.clear
                        .aspectRatio(1.2, contentMode: .fit)
                        .frame(maxWidth: .infinity)
                        .overlay { Image(uiImage: image).resizable().scaledToFit() }
                        .accessibilityLabel(frame.name)
                }
            }
            .padding(8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(chosen ? theme.colors.primaryContainer : theme.colors.surfaceVariant)
        }
        .buttonStyle(.plain)
        .task(id: frame.value) {
            let value = frame.value
            image = await offMain { frameImage(value) }
        }
    }
}
