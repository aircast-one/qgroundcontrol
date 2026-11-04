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

    private var forwarding: Bool { linksJson?["supportForwarding"].bool == true }
    private var host: Fact? { json.flatMap { $0["kind"].string == "object" ? factFromControl($0) : nil } }
    private var typed: String { host?.valueString ?? "" }

    var body: some View {
        NativeForm {
            Section {
                LabeledContent("Status", value: forwarding ? "Forwarding to support" : "Not forwarding")
            } footer: {
                Text("Sends live telemetry, including position, to an ArduPilot support engineer.")
            }
            if let host {
                Section {
                    FactRow(fact: host)
                } footer: {
                    if forwarding {
                        Text(FORWARDING_UNTIL_RESTART)
                    } else if verdict?.valid != true {
                        Text(verdict.flatMap { $0.error.isBlank ? nil : $0.error } ?? "Enter the address your support engineer gave you first.")
                    }
                }
            } else {
                Section {
                    Text("Reading the support address.").foregroundStyle(Color.secondary)
                }
            }
        }
        .pagePrimaryAction("Connect", enabled: host != nil && !forwarding && verdict?.valid == true) {
            offMain { LinkCommands.createSupportForwarding() }
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
