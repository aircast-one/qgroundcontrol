import SwiftUI

struct WifiNetwork: Equatable {
    let ssid: String
    let signal: Int
    let secured: Bool
    let active: Bool
}

struct WifiReopen: Equatable {
    let host: String
    let hostname: String
}

struct WifiJoinAttempt: Equatable {
    var attempt: Int
    var target: String
    var status: String
    var instructions: Bool
}

func parseWifiScan(_ json: JSON?) -> [WifiNetwork]? {
    guard let json else { return nil }
    return json["networks"].objects.map {
        WifiNetwork(ssid: $0["ssid"].string, signal: $0["signal"].int(0), secured: $0["secured"].bool, active: $0["active"].bool)
    }
}

func wifiJoinProblem(_ ssid: String, _ psk: String, _ pickedSecured: String) -> DeviceRefusal? {
    let name = ssid.trimmed
    if name.isEmpty { return DeviceRefusal(message: "Enter your network name first.", field: "ssid") }
    if psk.isEmpty && !pickedSecured.isEmpty && pickedSecured == name {
        return DeviceRefusal(message: "“\(name)” is a secured network — enter its password.", field: "psk")
    }
    if !psk.isEmpty && !(8...63).contains(psk.count) {
        return DeviceRefusal(message: "Password must be 8–63 characters, or blank for an open network.", field: "psk")
    }
    return nil
}

func joinOutcome(_ join: WifiJoinAttempt, _ refusal: DeviceRefusal?) -> WifiJoinAttempt {
    guard let refusal else {
        return withChanges(join) { $0.status = "✓ Connected to “\(join.target)” — it's now the active link." }
    }
    if refusal == DEVICE_NO_ANSWER {
        return withChanges(join) {
            $0.status = "Lost contact with the device — it may be switching networks. If it doesn't come back, reconnect using the steps below."
        }
    }
    let reason = refusal.message.ifEmpty("Couldn't join that network.")
    return withChanges(join) {
        $0.instructions = false
        $0.status = "✗ \(reason)"
    }
}

private let NO_JOIN = WifiJoinAttempt(attempt: 0, target: "", status: "", instructions: false)

struct WifiJoin: View {
    let reopen: WifiReopen
    var initiallyOpen: Bool = false

    var body: some View {
        DeviceDisclosure(
            "Join a Wi-Fi network",
            summary: "Connect this device to your Wi-Fi. It leaves the recovery hotspot and rejoins on every boot.",
            initiallyOpen: initiallyOpen
        ) {
            WifiWizard(reopen: reopen)
        }
    }
}

private struct WifiWizard: View {
    let reopen: WifiReopen
    @Environment(\.theme) private var theme
    @State private var joining = false
    @State private var join = NO_JOIN
    @State private var ssid = ""
    @State private var psk = ""
    @State private var pickedSecured = ""
    @State private var problem: DeviceRefusal?
    @State private var scanning = false
    @State private var scanFailed = false
    @State private var networks: [WifiNetwork]?

    private func scan() {
        scanning = true
        Task { @MainActor in
            let found = await deviceRead("/api/network/wifi/scan") { parseWifiScan($0) }
            scanning = false
            scanFailed = found == nil
            networks = found
        }
    }

    private func pick(_ network: WifiNetwork) {
        ssid = network.ssid
        psk = ""
        pickedSecured = network.secured ? network.ssid : ""
    }

    private func connect() {
        let refused = wifiJoinProblem(ssid, psk, pickedSecured)
        problem = refused
        guard refused == nil else { return }
        let target = ssid.trimmed
        let attempt = join.attempt + 1
        join = WifiJoinAttempt(attempt: attempt, target: target, status: "Switching to “\(target)”…", instructions: true)
        joining = true
        let body = deviceBody(["ssid": target, "psk": psk])
        Task { @MainActor in
            let refusal = await deviceCall(.POST, "/api/network/wifi", body)
            if join.attempt == attempt { join = joinOutcome(join, refusal) }
        }
    }

    private func back() {
        joining = false
        join.attempt += 1
    }

    var body: some View {
        if joining {
            WifiJoining(join: join, reopen: reopen, onBack: back)
        } else {
            VStack(alignment: .leading, spacing: 12) {
                HStack(spacing: 12) {
                    DeviceHint(text: "Pick a nearby network or type its name. Scanning is limited while the hotspot is on, so typing is usually fastest.")
                    Button(scanning ? "Scanning…" : "Scan", action: scan)
                        .buttonStyle(.bordered)
                        .disabled(scanning)
                }
                if scanFailed { DeviceHint(text: "Scan failed — type your network name below.") }
                if let found = networks {
                    if found.isEmpty { DeviceHint(text: "No networks found — type your network name below.") }
                    VStack(spacing: 6) {
                        ForEach(found, id: \.ssid) { network in
                            NetworkRow(network: network) { pick(network) }
                        }
                    }
                }
                // .password gives the plain keyboard without autocapitalisation or autocorrect, which an SSID needs.
                DeviceTextField(
                    label: "Network name (SSID)",
                    value: ssid,
                    onValueChange: { ssid = $0 },
                    placeholder: "your Wi-Fi name",
                    isError: problem?.field == "ssid",
                    keyboard: .password,
                    onSubmit: connect
                )
                SecretField(
                    label: "Password",
                    value: psk,
                    onValueChange: { psk = $0 },
                    isError: problem?.field == "psk",
                    supportingText: "Leave blank for an open network."
                )
                if let problem {
                    Text(problem.message).font(.bodyMedium).foregroundStyle(theme.colors.error)
                }
                HStack(spacing: 12) {
                    DeviceHint(text: "Saved with autoconnect, ahead of the hotspot.")
                    Button("Connect", action: connect).buttonStyle(.bordered)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, DEVICE_LIST_PADDING)
            .padding(.vertical, 8)
        }
    }
}

private struct NetworkRow: View {
    let network: WifiNetwork
    let onPick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onPick) {
            HStack(spacing: 12) {
                HStack(spacing: 8) {
                    Text(network.ssid).font(.bodyLarge).foregroundStyle(theme.colors.onSurface).lineLimit(1).truncationMode(.tail)
                    if network.active {
                        Text("connected").font(.labelSmall).foregroundStyle(theme.aircast.success)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                Text("\(network.signal)%").font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                if network.secured {
                    Image(.lock)
                        .font(.system(size: 14))
                        .foregroundStyle(theme.colors.onSurfaceVariant)
                        .accessibilityLabel("secured")
                }
            }
            .padding(.horizontal, 16)
            .frame(maxWidth: .infinity, minHeight: Metrics.rowCompact)
            .overlay(RoundedRectangle(cornerRadius: Corner.medium).stroke(theme.colors.outlineVariant))
            .contentShape(RoundedRectangle(cornerRadius: Corner.medium))
            .macHover(RoundedRectangle(cornerRadius: Corner.medium))
        }
        .buttonStyle(.plain)
    }
}

private struct WifiJoining: View {
    let join: WifiJoinAttempt
    let reopen: WifiReopen
    let onBack: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        let muted = theme.colors.onSurfaceVariant
        VStack(alignment: .leading, spacing: 12) {
            Text(join.status).font(.bodyLarge)
            if join.instructions {
                VStack(alignment: .leading, spacing: 8) {
                    Text("The recovery hotspot is turning off. To reach the device again:").font(.bodyMedium).foregroundStyle(muted)
                    (Text("1. On your phone or laptop, join ") + Text(join.target).bold().foregroundStyle(theme.colors.onSurface) + Text("."))
                        .font(.bodyMedium)
                        .foregroundStyle(muted)
                    Text("2. Reconnect this app to the device on that network.").font(.bodyMedium).foregroundStyle(muted)
                    ReconnectDevice(host: reopen.host, onConnected: onBack).id(reopen.host)
                    if !reopen.host.isEmpty {
                        DeviceHint(text: "Doesn't connect? Find \(reopen.hostname) in your router's list of connected devices and use its address instead.")
                    }
                    DeviceHint(text: "The app reaches the device over the link that's going away, so it can't follow it on its own.")
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 16)
                .padding(.vertical, 12)
                .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.medium))
            }
            Button("← Back", action: onBack).buttonStyle(.bordered)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, DEVICE_LIST_PADDING)
        .padding(.vertical, 8)
    }
}

private struct ReconnectDevice: View {
    let onConnected: () -> Void
    @State private var address: String
    @State private var busy = false
    @State private var refusal = ""

    init(host: String, onConnected: @escaping () -> Void) {
        self.onConnected = onConnected
        _address = State(initialValue: host)
    }

    private func reconnect() {
        guard !busy, !address.isBlank else { return }
        busy = true
        let target = address
        Task { @MainActor in
            refusal = await DeviceLink.connect(target) ?? ""
            busy = false
            if refusal.isEmpty { onConnected() }
        }
    }

    var body: some View {
        DeviceTextField(
            label: "Device address",
            value: address,
            onValueChange: { address = $0 },
            placeholder: "192.168.1.100 or falcon.local",
            isError: !refusal.isEmpty,
            supporting: refusal,
            keyboard: .uri,
            onSubmit: reconnect
        )
        Button("Reconnect", action: reconnect)
            .buttonStyle(.bordered)
            .disabled(busy || address.isBlank)
    }
}
