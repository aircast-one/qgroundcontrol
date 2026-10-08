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

private func objects<T>(_ list: JSON, _ read: (JSON) -> T) -> [T] {
    list.array.filter { $0.object != nil }.map(read)
}

func apmAirframe(_ view: JSON?) -> ApmAirframe? {
    guard let it = view, it["available"].bool else { return nil }
    return ApmAirframe(
        help: it["help"].string,
        frameType: it["frameType"].isNull ? nil : it["frameType"].int(0),
        invalidText: it["invalidText"].string,
        classes: objects(it["classes"]) { c in
            FrameClassCard(
                name: c["name"].string,
                value: c["value"].int(0),
                chosen: c["chosen"].bool,
                image: c["image"].string,
                types: objects(c["types"]) { FrameTypeChoice(name: $0["name"].string, value: $0["value"].int(0)) },
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
                Text("This page is for an ArduPilot vehicle with a FRAME_CLASS parameter.").padding(16)
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
        return ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                Text(state.help)
                    .font(.bodyMedium)
                    .padding(16)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.large))
                if let refusal {
                    Text(refusal).foregroundStyle(theme.colors.error)
                }
                if let card = chosen {
                    FrameTypeChips(types: card.types, frameType: state.frameType) { act(APM_AIRFRAME_PICK_TYPE, $0) }
                }
                LazyVGrid(columns: [GridItem(.adaptive(minimum: FRAME_TILE_MIN), spacing: 8)], spacing: 8) {
                    ForEach(state.classes, id: \.value) { card in
                        FrameClassTile(card: card, invalidText: state.invalidText) { act(APM_AIRFRAME_PICK_CLASS, card.value) }
                    }
                }
            }
            .padding(16)
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
                    if let asset = airframeImageAsset(card.image) {
                        Color.clear
                            .aspectRatio(1, contentMode: .fit)
                            .frame(maxWidth: .infinity)
                            .overlay { Image(asset).resizable().scaledToFit() }
                            .accessibilityLabel(card.name)
                    }
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

private struct FrameTypeChips: View {
    let types: [FrameTypeChoice]
    let frameType: Int?
    let onPick: (Int) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Frame type").font(.labelLarge).foregroundStyle(theme.colors.onSurfaceVariant)
            PlanFlowRow(spacing: 8) {
                ForEach(types, id: \.value) { type in
                    PlanChip(label: type.name, selected: type.value == frameType) { if type.value != frameType { onPick(type.value) } }
                }
            }
        }
    }
}
