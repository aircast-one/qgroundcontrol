import SwiftUI

let ESP_BRIDGE_VIEW = "view.espBridge"
let ESP_BRIDGE_SCREEN = "espBridge"
private let ESP_BRIDGE_OPENED = "espBridge.open"
private let ESP_POLL_MS = 1000
private let WIFI_MODES = ["Access point mode", "Station mode"]
private let STATION_MODE = 1
private let HOST_PORTS = 1024...65535

struct LinkCounts: Equatable {
    let received: String
    let lost: String
    let sent: String
}

struct EspBridge: Equatable {
    var modeIndex: Int?
    var modePath: String
    var channel: Int
    var channelPath: String
    var ssid: String
    var password: String
    var ssidSta: String?
    var passwordSta: String?
    var baudRates: [Int64]
    var baudIndex: Int
    var hostPort: String
    var hostPortPath: String
    var vehicle: LinkCounts
    var bridge: LinkCounts
    var qgc: LinkCounts
    var rebootPrompt: String
    var busy: Bool
}

private let GROUPED = IntegerFormatStyle<Int64>.number.locale(Locale(identifier: "en_US"))

func grouped(_ value: JSON?) -> String {
    guard case .number(let number)? = value, number.isFinite, abs(number) < 9.2e18 else { return "" }
    return Int64(number).formatted(GROUPED)
}

private func counts(_ json: JSON?) -> LinkCounts {
    LinkCounts(received: grouped(json?["received"]), lost: grouped(json?["lost"]), sent: grouped(json?["sent"]))
}

func espBridge(_ view: JSON?) -> EspBridge? {
    guard let it = view, it["available"].bool else { return nil }
    let status = it["status"]
    return EspBridge(
        modeIndex: it["modeIndex"].isNull ? nil : it["modeIndex"].int(0),
        modePath: it["modePath"].string,
        channel: it["channel"].int(1),
        channelPath: it["channelPath"].string,
        ssid: it["ssid"].string,
        password: it["password"].string,
        ssidSta: it["ssidSta"].isNull ? nil : it["ssidSta"].string,
        passwordSta: it["passwordSta"].isNull ? nil : it["passwordSta"].string,
        baudRates: it["baudRates"].array.map { $0.int64 ?? 0 },
        baudIndex: it["baudIndex"].int(4),
        hostPort: it["hostPort"]["valueString"].string,
        hostPortPath: it["hostPort"]["path"].string,
        vehicle: counts(status["vehicle"]),
        bridge: counts(status["bridge"]),
        qgc: counts(status["qgc"]),
        rebootPrompt: it["rebootPrompt"].string,
        busy: it["busy"].bool
    )
}

func stationFieldsEnabled(_ bridge: EspBridge) -> Bool { bridge.modeIndex == STATION_MODE }

func hostPortTyped(_ typed: String) -> Int? { Int(typed).flatMap { HOST_PORTS.contains($0) ? $0 : nil } }

struct EspBridgeScreen: View {
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var read: EspBridge?
    @State private var refusal: String?
    @State private var confirmReboot = false

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear
            if let bridge = read {
                screen(bridge)
            } else {
                NativeForm {
                    Section { Text("No WiFi bridge is connected.").foregroundStyle(Color.secondary) }
                }
            }
        }
        .task {
            _ = await offMain { Qgc.invoke(ESP_BRIDGE_OPENED) }
        }
        .task(id: revision) {
            let bridge = await offMain { espBridge(Qgc.get(ESP_BRIDGE_VIEW)) }
            read = bridge
            try? await Task.sleep(for: .milliseconds(ESP_POLL_MS))
            if !Task.isCancelled { revision += 1 }
        }
        .alert("Reboot WiFi bridge", isPresented: $confirmReboot) {
            Button("Yes") { act("espBridge.reboot") }
            Button("No", role: .cancel) {}
        } message: {
            Text(read?.rebootPrompt ?? "")
        }
    }

    private func screen(_ bridge: EspBridge) -> some View {
        NativeForm {
            Section {
                if let mode = bridge.modeIndex {
                    Choice(label: "WiFi mode", options: WIFI_MODES, index: mode) { set("\(bridge.modePath).rawValue", $0) }
                }
                Choice(label: "WiFi channel", options: (1...11).map(String.init), index: bridge.channel - 1, enabled: (bridge.modeIndex ?? 0) == 0) {
                    set("\(bridge.channelPath).rawValue", $0 + 1)
                }
                TextSetting(label: "WiFi AP SSID", value: bridge.ssid) { act("espBridge.setText", "ssid", $0) }
                TextSetting(label: "WiFi AP password", value: bridge.password) { act("espBridge.setText", "password", $0) }
                TextSetting(label: "WiFi STA SSID", value: bridge.ssidSta ?? "", enabled: stationFieldsEnabled(bridge)) { act("espBridge.setText", "ssidSta", $0) }
                TextSetting(label: "WiFi STA password", value: bridge.passwordSta ?? "", enabled: stationFieldsEnabled(bridge)) { act("espBridge.setText", "passwordSta", $0) }
                Choice(label: "UART baud rate", options: bridge.baudRates.map { String($0) }, index: bridge.baudIndex) { act("espBridge.baud", $0) }
                TextSetting(label: "QGC UDP port", value: bridge.hostPort, enabled: !bridge.hostPortPath.isEmpty, digits: true) { typed in
                    if let port = hostPortTyped(typed) { set(bridge.hostPortPath, port) }
                }
                if let refusal {
                    Text(refusal).foregroundStyle(theme.colors.error)
                }
            } header: {
                SectionHeader(text: "ESP WiFi bridge settings")
            }
            ForEach([("Bridge/vehicle link", bridge.vehicle), ("Bridge/QGC link", bridge.bridge), ("QGC/bridge link", bridge.qgc)], id: \.0) { title, link in
                Section {
                    LabeledContent("Messages received", value: link.received)
                    LabeledContent("Messages lost", value: link.lost)
                    LabeledContent("Messages sent", value: link.sent)
                } header: {
                    SectionHeader(text: title)
                }
            }
            Section {
                Button("Reset counters") { act("espBridge.resetCounters") }
                Button("Restart WiFi bridge") { confirmReboot = true }.disabled(bridge.busy)
            }
            Section {
                Button("Restore defaults", role: .destructive) { act("espBridge.restoreDefaults") }
            }
        }
    }

    private func act(_ path: String, _ args: Any...) {
        Task { refusal = await offMain { Qgc.refusalOf(path, arguments: args) } }
    }

    private func set(_ path: String, _ value: Any) {
        Task { refusal = await offMain { Qgc.writeRefusal(path, value) } }
    }
}

private struct Choice: View {
    let label: String
    let options: [String]
    let index: Int
    var enabled = true
    let onPick: (Int) -> Void

    var body: some View {
        Picker(label, selection: Binding(get: { index }, set: { picked in if picked != index { onPick(picked) } })) {
            ForEach(Array(options.enumerated()), id: \.offset) { at, option in
                Text(option).tag(at)
            }
        }
        .disabled(!enabled)
    }
}

private struct TextSetting: View {
    let label: String
    let value: String
    var enabled = true
    var digits = false
    let onDone: (String) -> Void
    @State private var typed = ""

    var body: some View {
        FieldRow(
            label: label,
            value: typed,
            enabled: enabled,
            keyboard: digits ? .numbersAndPunctuation : .default,
            onSubmit: { onDone(typed) },
            onChange: { next in
                typed = digits ? String(next.filter { $0.isASCII && $0.isNumber }.prefix(5)) : String(next.prefix(16))
            }
        )
        .onChange(of: value, initial: true) { typed = value }
    }
}
