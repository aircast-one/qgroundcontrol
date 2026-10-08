import SwiftUI

let GENERAL_PAGE = "General"
let CLEAR_SETTINGS_NEXT_BOOT = "settings.appSettings.clearSettingsNextBoot"

func resetPending(_ value: JSON?) -> Bool { value == .bool(true) || value == .number(1) || value == .string("true") }

struct ResetAllSettingsRow: View {
    @State private var reads = 0
    @State private var pending = false
    @State private var asking = false

    var body: some View {
        HStack {
            Text(pending ? "All settings will be cleared on next start" : "")
                .font(.bodySmall)
                .frame(maxWidth: .infinity, alignment: .leading)
            Button(pending ? "Cancel reset" : "Reset all settings…") {
                if pending { write(false) } else { asking = true }
            }
            .buttonStyle(.bordered)
        }
        .padding(.horizontal, Space.s5)
        .padding(.vertical, Space.s3)
        .task(id: reads) {
            pending = await offMain { resetPending(Qgc.get(CLEAR_SETTINGS_NEXT_BOOT)["value"]) }
        }
        .alert("Reset all settings", isPresented: $asking) {
            Button("Ok") { write(true) }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("All settings will be cleared the next time Aircast starts. This cannot be undone.")
        }
    }

    private func write(_ value: Bool) {
        Task {
            _ = await offMain { Qgc.set(CLEAR_SETTINGS_NEXT_BOOT, value) }
            reads += 1
        }
    }
}
