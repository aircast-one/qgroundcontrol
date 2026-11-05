import SwiftUI

func breachAltitudeText(_ altitude: Double?) -> String { altitude.map { String(format: "%.1f", $0) } ?? "" }

struct BreachReturnDialog: View {
    let breach: BreachReturn
    let onDismiss: () -> Void
    let onAltitude: (Double) -> Void
    let onRemove: () -> Void
    @State private var typed = ""

    var body: some View {
        let value = typedNumber(typed)
        NativeDialogSheet(title: "Breach return point", onDismissRequest: onDismiss, detents: [.medium]) {
            NativeForm {
                Section {
                    FieldRow(
                        label: breach.units.isBlank ? "Altitude" : "Altitude (\(breach.units))",
                        value: typed,
                        error: value == nil ? "Enter a number" : nil,
                        keyboard: .numbersAndPunctuation,
                        onChange: { typed = $0 }
                    )
                }
                Section {
                    Button("Remove breach return point", role: .destructive, action: onRemove)
                }
            }
            .dialogButtons(
                cancel: DialogButton(title: "Cancel", action: onDismiss),
                primary: DialogButton(title: "Set", enabled: value != nil && !breach.altitudePath.isEmpty) { if let value { onAltitude(value) } }
            )
        }
        .onChange(of: breach, initial: true) { typed = breachAltitudeText(breach.altitude) }
    }
}
