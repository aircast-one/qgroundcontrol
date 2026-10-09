import SwiftUI

func breachAltitudeText(_ altitude: Double?) -> String { altitude.map { String(format: "%.1f", $0) } ?? "" }

struct BreachReturnDialog: View {
    let breach: BreachReturn
    let onDismiss: () -> Void
    let onAltitude: (Double) -> Void
    let onRemove: () -> Void
    @State private var typed = ""
    @Environment(\.theme) private var theme

    var body: some View {
        let value = typedNumber(typed)
        PlanDialog(title: "Breach return point", onDismiss: onDismiss) {
            VStack(alignment: .leading, spacing: 8) {
                PlanTextField(label: "Altitude", text: $typed, suffix: breach.units, isError: value == nil)
                Button(action: onRemove) {
                    Text("Remove breach return point").foregroundStyle(theme.colors.error)
                }
                .buttonStyle(.text)
            }
        } buttons: {
            Button("Close", action: onDismiss)
            Button("Set") { value.map(onAltitude) }.disabled(value == nil || breach.altitudePath.isEmpty)
        }
        .onChange(of: breach, initial: true) { typed = breachAltitudeText(breach.altitude) }
    }
}
