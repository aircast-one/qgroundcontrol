import SwiftUI

private let HOST_FACT = "settings.mavlinkSettings.forwardMavlinkAPMSupportHostName"

let SUPPORT_HOST_DEBOUNCE_MS = 250

let FORWARDING_UNTIL_RESTART = "Forwarding traffic: Mavlink traffic will keep being forwarded until application restarts"

struct SupportHostVerdict: Equatable {
    let valid: Bool
    let error: String
}

func supportHostPath(_ host: String) -> String { "view.supportHost(\(host))" }

func supportHostCannotBeAsked(_ host: String) -> String? {
    if host.isBlank { return nil }
    return host != host.trimmed ? "Remove the space before or after the address." : nil
}

func supportHostVerdict(_ view: JSON?) -> SupportHostVerdict? {
    guard let view, view["class"].string == "SupportHost" else { return nil }
    return SupportHostVerdict(valid: view["valid"].bool, error: view["error"].string)
}

struct RemoteSupportScreen: View {
    @QgcPath("view.links") private var linksJson
    @QgcPath(settingControl(HOST_FACT)) private var json
    @State private var verdict: SupportHostVerdict?
    @Environment(\.theme) private var theme

    private var forwarding: Bool { linksJson?["supportForwarding"].bool == true }
    private var host: Fact? { json.flatMap { $0["kind"].string == "object" ? factFromControl($0) : nil } }
    private var typed: String { host?.valueString ?? "" }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                EmptyState(
                    icon: .link,
                    title: forwarding ? "Forwarding to support" : "Not forwarding",
                    text: "Sends live telemetry, including position, to an ArduPilot support engineer."
                )
                if let host {
                    FactRow(fact: host)
                    if !forwarding && verdict?.valid != true {
                        Text(verdict.flatMap { $0.error.isBlank ? nil : $0.error } ?? "Enter the address your support engineer gave you first.")
                            .font(.bodySmall)
                            .foregroundStyle(theme.colors.onSurfaceVariant)
                            .padding(.horizontal, 32)
                    }
                    HStack {
                        Spacer()
                        Button("Connect") { offMain { LinkCommands.createSupportForwarding() } }
                            .buttonStyle(.borderedProminent)
                            .disabled(forwarding || verdict?.valid != true)
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                    if forwarding {
                        Text(FORWARDING_UNTIL_RESTART).font(.bodyMedium).padding(.horizontal, 16)
                    }
                } else {
                    Text("Reading the support address.").font(.bodyMedium).padding(.horizontal, 16)
                }
            }
        }
        .task(id: typed) {
            let asked = typed
            if let refused = supportHostCannotBeAsked(asked) {
                verdict = SupportHostVerdict(valid: false, error: refused)
                return
            }
            guard (try? await Task.sleep(for: .milliseconds(SUPPORT_HOST_DEBOUNCE_MS))) != nil else { return }
            verdict = await offMain { supportHostVerdict(Qgc.get(supportHostPath(asked))) }
        }
    }
}
