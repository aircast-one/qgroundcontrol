import SwiftUI

let CREATE_FROM_TEMPLATE = "plan.createFromTemplate"
private let BLANK_LABEL = "Blank mission"

struct PlanTemplatesState: Equatable {
    var enabled: Bool
    var names: [String]
    var blank: String
}

func planTemplates(_ view: JSON?) -> PlanTemplatesState? {
    guard let templates = view?["templates"], templates.object != nil else { return nil }
    return PlanTemplatesState(
        enabled: templates["enabled"].bool,
        names: templates["names"].strings,
        blank: templates["blank"].string
    )
}

func templateChoices(_ state: PlanTemplatesState?) -> [(String?, String)] {
    [(nil, BLANK_LABEL)] + (state?.names ?? []).filter { $0 != state?.blank }.map { ($0, sentenceCase($0)) }
}

struct NewPlanDialog: View {
    let state: PlanTemplatesState?
    let replacing: Bool
    let onDismiss: () -> Void
    let onPick: (String?) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        PlanDialog(title: "New plan", onDismiss: onDismiss) {
            VStack(alignment: .leading, spacing: Space.s2) {
                Text(replacing ? "Replaces the plan you are editing. Templates start at the map centre." : "Templates start at the map centre.")
                    .font(.bodyMedium)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                ForEach(templateChoices(state), id: \.0) { name, label in
                    Button(label) { onPick(name) }
                        .buttonStyle(.text)
                        .frame(minHeight: 40)
                        .disabled(!(name == nil || state?.enabled == true))
                }
            }
        } buttons: {
            Button("Cancel", action: onDismiss)
        }
    }
}
