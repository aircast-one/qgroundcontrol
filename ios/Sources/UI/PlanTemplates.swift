import SwiftUI

let CREATE_FROM_TEMPLATE = "plan.createFromTemplate"
private let BLANK_LABEL = "Blank mission"
private let FIRST_STEP = "Tap the map to set home, or start from a template"
private let DISABLED_ALPHA = 0.5
private let TEMPLATE_PANEL_WIDTH: CGFloat = 380

func touchWording(_ prompt: String) -> String {
    prompt.replacing(#/\b([Cc])lick/#) { match in match.output.1 == "C" ? "Tap" : "tap" }
}

struct PlanTemplatesState: Equatable {
    var show: Bool
    var enabled: Bool
    var prompt: String
    var names: [String]
    var homeSet: Bool
    var blank: String
}

func planTemplates(_ view: JSON?) -> PlanTemplatesState? {
    guard let templates = view?["templates"], templates.object != nil else { return nil }
    return PlanTemplatesState(
        show: templates["show"].bool,
        enabled: templates["enabled"].bool,
        prompt: templates["prompt"].string,
        names: templates["names"].strings,
        homeSet: templates["homeSet"].bool,
        blank: templates["blank"].string
    )
}

struct PlanTemplates: View {
    let planStatus: JSON?
    let centre: (Double, Double)?
    let onRefused: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        if let state = planTemplates(planStatus), state.show {
            VStack(alignment: .leading, spacing: Space.s2) {
                Text(templatePrompt(state))
                    .font(.bodyMedium)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .padding(.horizontal, Space.s5)
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: Space.s2) {
                        ForEach(templateChoices(state), id: \.0) { name, label in
                            Button { create(name) } label: {
                                Text(label)
                                    .font(.labelLarge)
                                    .lineLimit(1)
                                    .padding(.horizontal, Space.s4)
                                    .padding(.vertical, Space.s2)
                                    .foregroundStyle(theme.colors.onSecondaryContainer)
                                    .background(theme.colors.secondaryContainer, in: Capsule())
                            }
                            .buttonStyle(.plain)
                            .disabled(!state.enabled)
                            .opacity(state.enabled ? 1 : DISABLED_ALPHA)
                        }
                    }
                    .padding(.horizontal, Space.s4)
                }
            }
            .padding(.vertical, Space.s3)
            .frame(maxWidth: TEMPLATE_PANEL_WIDTH, alignment: .leading)
            .background(theme.colors.surfaceContainer, in: RoundedRectangle(cornerRadius: Corner.extraLarge))
        }
    }

    private func create(_ name: String) {
        guard let (lat, lon) = centre else { return onRefused("The map has not reported its centre yet.") }
        Task {
            if let refused = await offMain({ Qgc.refusalOf(CREATE_FROM_TEMPLATE, name, lat, lon) }) {
                onRefused(refused)
            }
        }
    }
}

func templateChoices(_ state: PlanTemplatesState) -> [(String, String)] {
    state.names.filter { $0 == state.blank }.map { ($0, BLANK_LABEL) }
        + state.names.filter { $0 != state.blank }.map { ($0, sentenceCase($0)) }
}

func templatePrompt(_ state: PlanTemplatesState) -> String {
    state.homeSet ? touchWording(state.prompt) : FIRST_STEP
}
