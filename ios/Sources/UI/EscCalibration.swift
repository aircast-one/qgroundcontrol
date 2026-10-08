import SwiftUI

let DIALOG_CONTROL = "dialog"
private let ESC_CAL_VIEW = "view.escCalibration"
private let ESC_CAL_POLL_MS = 500

struct EscCalibrationState: Equatable {
    var highlight: String
    var text: String
    var running: Bool = false
}

func escCalibration(_ view: JSON?) -> EscCalibrationState? {
    guard let view, view["open"].bool else { return nil }
    return EscCalibrationState(highlight: view["highlight"].string, text: view["text"].string, running: view["running"].bool)
}

struct EscCalibrationDialog: View {
    let onClose: () -> Void
    @State private var state: EscCalibrationState?
    @Environment(\.theme) private var theme

    var body: some View {
        SetupDialog(title: "ESC Calibration") {
            Text(state?.highlight ?? "").bold().foregroundStyle(theme.colors.error)
                + Text(state?.text ?? "Starting ESC calibration...")
        } buttons: {
            Button("OK") {
                offMain { _ = SetupCommands.closeEscCalibration() }
                onClose()
            }
            .buttonStyle(.borderedProminent)
            .disabled(state?.running != false)
        }
        .interactiveDismissDisabled()
        .task {
            _ = await offMain { SetupCommands.startEscCalibration() }
            while !Task.isCancelled {
                state = await offMain { escCalibration(Qgc.get(ESC_CAL_VIEW)) }
                try? await Task.sleep(for: .milliseconds(ESC_CAL_POLL_MS))
            }
        }
    }
}
