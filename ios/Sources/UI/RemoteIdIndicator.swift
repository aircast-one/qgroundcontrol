import SwiftUI

let REMOTE_ID_STATUS_PATH = "view.remoteIdStatus"
let REMOTE_ID_SETTINGS_PAGE = "Remote ID"
private let SEND_SELF_ID = "sendSelfID"
private let SELF_ID_FACTS = [SEND_SELF_ID, "selfIDType", "selfIDFree", "selfIDExtended", "selfIDEmergency"]
private let BROADCAST_GATED: Set<String> = ["selfIDType", "selfIDFree", "selfIDExtended"]
private let SELF_ID_LABELS = [SEND_SELF_ID: "Broadcast", "selfIDType": "Broadcast message"]
private let DEFAULT_EMERGENCY_HOLD_MS: Int64 = 800
private let SELF_ID_NOTE = "If an emergency is declared, Emergency Text will be broadcast even if Broadcast setting is not enabled."

func selfIdFacts(_ page: [Fact]) -> [Fact] {
    let byName = Dictionary(page.map { ($0.name, $0) }, uniquingKeysWith: { _, last in last })
    let broadcasting = byName[SEND_SELF_ID]?.boolValue == true
    return SELF_ID_FACTS.compactMap { byName[$0] }
        .map { fact in
            guard let label = SELF_ID_LABELS[fact.name] else { return fact }
            var named = fact
            named.description = label
            return named
        }
        .map { fact in
            guard BROADCAST_GATED.contains(fact.name), !broadcasting else { return fact }
            var gated = fact
            gated.enabled = false
            gated.disabledReason = "Broadcast is off"
            return gated
        }
}

struct RemoteIdStatus: Equatable {
    var state: String
    var comms: Bool
    var arm: Bool
    var armError: String
    var gps: Bool
    var basicId: Bool
    var operatorIdShown: Bool
    var operatorId: Bool
    var emergency: Bool
    var holdMs: Int64
}

func remoteIdStatus(_ view: JSON?) -> RemoteIdStatus? {
    guard let it = view, it["shown"].bool else { return nil }
    return RemoteIdStatus(
        state: it["state"].string,
        comms: it["comms"].bool,
        arm: it["armStatus"].bool,
        armError: it["armStatusError"].string,
        gps: it["gcsGps"].bool,
        basicId: it["basicId"].bool,
        operatorIdShown: it["operatorIdShown"].bool,
        operatorId: it["operatorId"].bool,
        emergency: it["emergency"].bool,
        holdMs: it["emergencyHoldMs"].int64 ?? DEFAULT_EMERGENCY_HOLD_MS
    )
}

func remoteIdRows(_ status: RemoteIdStatus) -> [(String, Bool)] {
    [
        (status.comms ? "RID COMMS" : "NOT CONNECTED", status.comms),
        status.comms ? ("ARM STATUS", status.arm) : nil,
        status.comms ? ("GCS GPS", status.gps) : nil,
        status.comms ? ("BASIC ID", status.basicId) : nil,
        status.comms && status.operatorIdShown ? ("OPERATOR ID", status.operatorId) : nil,
    ].compactMap { $0 }
}

private func stateColour(_ state: String, _ theme: Theme) -> Color {
    switch state {
    case "healthy": theme.aircast.success
    case "warning": theme.aircast.warning
    case "error": theme.colors.error
    default: theme.colors.onSurfaceVariant
    }
}

struct RemoteIdIndicatorCell: View {
    @Environment(\.theme) private var theme
    @Environment(AppNavigationState.self) private var navigation
    @AdvancedUiShown private var advanced
    @QgcPath(REMOTE_ID_STATUS_PATH) private var view

    var body: some View {
        if let status = remoteIdStatus(view) {
            StatusCellSheet {
                Text("RID")
                    .font(.labelMedium)
                    .foregroundStyle(stateColour(status.state, theme))
                    .lineLimit(1)
            } sheet: { close in
                ScrollView {
                    RemoteIdStatusPage(status: status, advanced: advanced, onConfigure: {
                        navigation.settingsPage = REMOTE_ID_SETTINGS_PAGE
                        close()
                    })
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, Space.s5)
                    .padding(.bottom, Space.s6)
                }
            }
        }
    }
}

private struct RemoteIdStatusPage: View {
    let status: RemoteIdStatus
    let advanced: Bool
    let onConfigure: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            Text("RemoteID Status").font(.titleMedium)
            ForEach(remoteIdRows(status), id: \.0) { label, good in
                Text(label)
                    .font(.labelLarge)
                    .foregroundStyle(good ? theme.aircast.success : theme.colors.error)
                    .onTapGesture(perform: onConfigure)
            }
            if !status.armError.isBlank {
                Text("Arm Status Error  \(status.armError)").font(.bodySmall)
            }
            if status.comms {
                Text(status.emergency ? "EMERGENCY HAS BEEN DECLARED, Press and Hold for 3 seconds to cancel" : "Press and Hold below button to declare emergency")
                    .font(.bodySmall)
                Text(status.emergency ? "Clear Emergency" : "EMERGENCY")
                    .font(.titleMedium)
                    .foregroundStyle(theme.colors.onErrorContainer)
                    .frame(maxWidth: .infinity)
                    .padding(Space.s4)
                    .background(theme.colors.errorContainer, in: RoundedRectangle(cornerRadius: Corner.medium))
                    .onLongPressGesture(minimumDuration: Double(status.holdMs) / 1000) {
                        let declare = !status.emergency
                        offMain { VehicleCommands.declareRemoteIdEmergency(declare) }
                    }
            }
            SelfIdSection()
            if advanced {
                HStack {
                    Text("Remote ID").frame(maxWidth: .infinity, alignment: .leading)
                    Button("Configure", action: onConfigure).buttonStyle(.bordered)
                }
            }
        }
    }
}

private struct SelfIdSection: View {
    @State private var reloads = 0
    @State private var facts: [Fact] = []

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            if !facts.isEmpty {
                Text("Self ID").font(.titleSmall)
                if facts.first(where: { $0.name == SEND_SELF_ID })?.boolValue != true {
                    Text(SELF_ID_NOTE).font(.bodySmall)
                }
                ForEach(facts) { fact in
                    FactRow(fact: fact, onWrite: { reloads += 1 })
                }
            }
        }
        .task(id: reloads) {
            facts = await offMain {
                selfIdFacts(settingsSections(Qgc.get(settingsPagePath(REMOTE_ID_SETTINGS_PAGE))).flatMap { section in section.blocks.flatMap(\.facts) })
            }
        }
    }
}
