import SwiftUI

let APM_AIRFRAME_SCREEN = "apmAirframe"
let APM_AIRFRAME_VIEW = "view.apmAirframe"
let APM_AIRFRAME_PICK_CLASS = "apmAirframe.pickClass"
let APM_AIRFRAME_PICK_TYPE = "apmAirframe.pickType"
private let APM_AIRFRAME_POLL_MS = 1000

struct FrameTypeChoice: Equatable {
    let name: String
    let value: Int
}

struct FrameClassCard: Equatable {
    let name: String
    let value: Int
    let chosen: Bool
    let image: String
    let types: [FrameTypeChoice]
    let valid: Bool
}

struct ApmAirframe: Equatable {
    let help: String
    let frameType: Int?
    let invalidText: String
    let classes: [FrameClassCard]
}

func apmAirframe(_ view: JSON?) -> ApmAirframe? {
    guard let it = view, it["available"].bool else { return nil }
    return ApmAirframe(
        help: it["help"].string,
        frameType: it["frameType"].isNull ? nil : it["frameType"].int(0),
        invalidText: it["invalidText"].string,
        classes: it["classes"].objects.map { c in
            FrameClassCard(
                name: c["name"].string,
                value: c["value"].int(0),
                chosen: c["chosen"].bool,
                image: c["image"].string,
                types: c["types"].objects.map { FrameTypeChoice(name: $0["name"].string, value: $0["value"].int(0)) },
                valid: c["valid"].bool(true)
            )
        }
    )
}

struct ApmAirframeScreen: View {
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var read: ApmAirframe?
    @State private var refusal: String?

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear
            if let state = read {
                screen(state)
            } else {
                NativeForm {
                    Section { Text("This page is for an ArduPilot vehicle with a FRAME_CLASS parameter.").foregroundStyle(Color.secondary) }
                }
            }
        }
        .task(id: revision) {
            let airframe = await offMain { apmAirframe(Qgc.get(APM_AIRFRAME_VIEW)) }
            read = airframe
            try? await Task.sleep(for: .milliseconds(APM_AIRFRAME_POLL_MS))
            if !Task.isCancelled { revision += 1 }
        }
    }

    private func screen(_ state: ApmAirframe) -> some View {
        let chosen = state.classes.first { $0.chosen && $0.valid && !$0.types.isEmpty }
        return NativeForm {
            if !state.help.isBlank {
                Section {
                    Text(state.help).font(.subheadline).foregroundStyle(Color.secondary)
                }
            }
            if let refusal {
                Section {
                    Text(refusal).foregroundStyle(theme.colors.error)
                }
            }
            if let card = chosen {
                Section {
                    Picker("Frame type", selection: Binding(get: { state.frameType ?? NO_FRAME_TYPE }, set: { picked in
                        if picked != state.frameType && picked != NO_FRAME_TYPE { act(APM_AIRFRAME_PICK_TYPE, picked) }
                    })) {
                        if state.frameType == nil || !card.types.contains(where: { $0.value == state.frameType }) {
                            Text("Not set").tag(NO_FRAME_TYPE)
                        }
                        ForEach(card.types, id: \.value) { type in
                            Text(type.name).tag(type.value)
                        }
                    }
                }
            }
            Section {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: FRAME_TILE_MIN), spacing: 8)], spacing: 8) {
                    ForEach(state.classes, id: \.value) { card in
                        FrameClassTile(card: card, invalidText: state.invalidText) { act(APM_AIRFRAME_PICK_CLASS, card.value) }
                    }
                }
                .padding(.vertical, Space.s1)
            } header: {
                SectionHeader(text: "Frame class")
            }
        }
    }

    private func act(_ path: String, _ value: Int) {
        Task {
            refusal = await offMain { Qgc.refusalOf(path, value) }
            read = await offMain { apmAirframe(Qgc.get(APM_AIRFRAME_VIEW)) }
        }
    }
}

private let FRAME_TILE_MIN: CGFloat = 104

// The frame-type picker's entry while FRAME_TYPE holds no type this frame class offers.
private let NO_FRAME_TYPE = Int.min

private struct FrameClassTile: View {
    let card: FrameClassCard
    let invalidText: String
    let onPickClass: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button {
            if !card.chosen || !card.valid { onPickClass() }
        } label: {
            ZStack {
                VStack(spacing: 4) {
                    Color.clear
                        .aspectRatio(1, contentMode: .fit)
                        .frame(maxWidth: .infinity)
                        .overlay {
                            if let asset = airframeImageAsset(card.image) {
                                Image(asset).resizable().scaledToFit()
                            }
                        }
                        .accessibilityLabel(card.name)
                    Text(card.name).font(.labelLarge).lineLimit(1)
                }
                .padding(8)
                .opacity(card.valid ? 1 : 0.5)
                if !card.valid {
                    Text(invalidText)
                        .font(.labelSmall)
                        .foregroundStyle(theme.colors.error)
                        .multilineTextAlignment(.center)
                        .padding(4)
                }
            }
            .frame(maxWidth: .infinity)
            .background(card.chosen ? theme.colors.primaryContainer : theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.large))
            .overlay {
                if card.chosen {
                    RoundedRectangle(cornerRadius: Corner.large).stroke(theme.colors.primary, lineWidth: 2)
                }
            }
        }
        .buttonStyle(.plain)
    }
}
