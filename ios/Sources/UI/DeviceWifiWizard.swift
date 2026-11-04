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
        DeviceGroup(
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
            // Rows inside the Join group: scan, the networks found (tap one to fill the name), name, password, Connect.
            HStack(spacing: Space.s3) {
                DeviceHint(text: "Pick a nearby network or type its name. Scanning is limited while the hotspot is on, so typing is usually fastest.")
                Button(scanning ? "Scanning…" : "Scan", action: scan)
                    .buttonStyle(.bordered)
                    .disabled(scanning)
            }
            .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
            if scanFailed { DeviceHint(text: "Scan failed — type your network name below.").rowPadding(horizontal: DEVICE_LIST_PADDING) }
            if let found = networks {
                if found.isEmpty { DeviceHint(text: "No networks found — type your network name below.").rowPadding(horizontal: DEVICE_LIST_PADDING) }
                ForEach(found, id: \.ssid) { network in
                    NetworkRow(network: network, picked: !ssid.isEmpty && ssid == network.ssid) { pick(network) }
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
            .rowPadding(horizontal: DEVICE_LIST_PADDING)
            SecretField(
                label: "Password",
                value: psk,
                onValueChange: { psk = $0 },
                isError: problem?.field == "psk",
                supportingText: "Leave blank for an open network."
            )
            .rowPadding(horizontal: DEVICE_LIST_PADDING)
            if let problem {
                Text(problem.message).font(.bodyMedium).foregroundStyle(theme.colors.error).rowPadding(horizontal: DEVICE_LIST_PADDING)
            }
            HStack(spacing: Space.s3) {
                DeviceHint(text: "Saved with autoconnect, ahead of the hotspot.")
                Button("Connect", action: connect).buttonStyle(.borderedProminent)
            }
            .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
        }
    }
}

private struct NetworkRow: View {
    let network: WifiNetwork
    var picked: Bool = false
    let onPick: () -> Void
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            // A native row: the name (and whether it is the one in use), its signal and lock; a check on the picked one.
            Button(action: onPick) {
                HStack(spacing: Space.s3) {
                    Image(systemName: "checkmark")
                        .fontWeight(.semibold)
                        .foregroundStyle(theme.colors.primary)
                        .opacity(picked ? 1 : 0)
                        .accessibilityHidden(!picked)
                    HStack(spacing: Space.s2) {
                        Text(network.ssid).foregroundStyle(Color.primary).lineLimit(1).truncationMode(.tail)
                        if network.active {
                            Text("connected").font(.footnote).foregroundStyle(theme.aircast.success)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    Text("\(network.signal)%").font(.footnote).foregroundStyle(Color.secondary)
                    if network.secured {
                        Image(systemName: "lock.fill")
                            .font(.footnote)
                            .foregroundStyle(Color.secondary)
                            .accessibilityLabel("secured")
                    }
                }
                .contentShape(Rectangle())
            }
            .listRowButton()
            .accessibilityAddTraits(picked ? .isSelected : [])
        } else {
            outlined
        }
    }

    private var outlined: some View {
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
        .padding(.horizontal, DEVICE_LIST_PADDING)
    }
}

private struct WifiJoining: View {
    let join: WifiJoinAttempt
    let reopen: WifiReopen
    let onBack: () -> Void
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList

    // Rows while the device switches networks: what happened, how to reach it again (with the address to reconnect
    // to), and Back to the form.
    var body: some View {
        let muted = nativeList ? Color.secondary : theme.colors.onSurfaceVariant
        Text(join.status).font(.bodyLarge).frame(maxWidth: .infinity, alignment: .leading).rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
        if join.instructions {
            VStack(alignment: .leading, spacing: 8) {
                Text("The recovery hotspot is turning off. To reach the device again:").font(.bodyMedium).foregroundStyle(muted)
                (Text("1. On your phone or laptop, join ") + Text(join.target).bold().foregroundStyle(nativeList ? Color.primary : theme.colors.onSurface) + Text("."))
                    .font(.bodyMedium)
                    .foregroundStyle(muted)
                Text("2. Reconnect this app to the device on that network.").font(.bodyMedium).foregroundStyle(muted)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
            ReconnectDevice(host: reopen.host, onConnected: onBack).id(reopen.host)
            if !reopen.host.isEmpty {
                DeviceHint(text: "Doesn't connect? Find \(reopen.hostname) in your router's list of connected devices and use its address instead.")
                    .rowPadding(horizontal: DEVICE_LIST_PADDING)
            }
            DeviceHint(text: "The app reaches the device over the link that's going away, so it can't follow it on its own.")
                .rowPadding(horizontal: DEVICE_LIST_PADDING)
        }
        Button("Back", action: onBack).buttonStyle(.bordered).rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
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
        .rowPadding(horizontal: DEVICE_LIST_PADDING)
        Button("Reconnect", action: reconnect)
            .buttonStyle(.borderedProminent)
            .disabled(busy || address.isBlank)
            .rowPadding(horizontal: DEVICE_LIST_PADDING)
    }
}
