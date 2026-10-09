import SwiftUI

struct JoystickDetail: Equatable {
    let label: String
    let value: String
    let warn: Bool
}

struct JoystickBadge: Equatable {
    let heading: String
    let enabledText: String
    let warn: Bool
    let typeText: String
    let inputsText: String
    var details: [JoystickDetail] = []
}

func joystickBadge(_ view: JSON?) -> JoystickBadge? {
    guard let it = view?["indicator"], it.object != nil else { return nil }
    return JoystickBadge(
        heading: it["heading"].string,
        enabledText: it["enabledText"].string,
        warn: it["warn"].bool,
        typeText: it["typeText"].string,
        inputsText: it["inputsText"].string,
        details: it["details"].objects.map { JoystickDetail(label: $0["label"].string, value: $0["value"].string, warn: $0["warn"].bool) }
    )
}

struct JoystickIndicatorCell: View {
    @Environment(\.theme) private var theme
    @QgcPath(JOYSTICK_VIEW) private var view
    @State private var open = false

    var body: some View {
        if let badge = joystickBadge(view) {
            let tint: Color? = badge.warn ? theme.aircast.warning : nil
            Text("Joystick")
                .font(.labelMedium)
                .foregroundOrInherited(tint)
                .lineLimit(1)
                .onTapGesture { open = true }
                .background {
                    if open {
                        AircastSheet(onDismissRequest: { open = false }) {
                            VStack(alignment: .leading, spacing: Space.s2) {
                                Text(badge.heading).font(.titleMedium)
                                Text("Enabled:  \(badge.enabledText)").font(.bodyMedium).foregroundOrInherited(tint)
                                Text("Type:  \(badge.typeText)").font(.bodyMedium)
                                Text("Inputs:  \(badge.inputsText)").font(.bodyMedium)
                                ForEach(Array(badge.details.enumerated()), id: \.offset) { _, detail in
                                    Text("\(detail.label)  \(detail.value)").font(.bodyMedium).foregroundOrInherited(detail.warn ? theme.colors.error : nil)
                                }
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.horizontal, Space.s5)
                            .padding(.bottom, Space.s6)
                        }
                    }
                }
        }
    }
}
