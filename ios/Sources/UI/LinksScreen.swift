import SwiftUI
import UniformTypeIdentifiers

private let LINKS_VIEW = "view.links"
private let LINKS_PATH = "links.linkConfigurations"
private let DEFAULT_PORT = "14550"
private let DEFAULT_TCP_PORT = "5760"
private let UDP_LISTEN_PORT = "settings.autoConnectSettings.udpListenPort"
private let HOST_CANNOT_OPEN: Set<LinkType> = [.Serial, .Bluetooth]

enum LinkType: String, CaseIterable {
    case Udp = "udp"
    case Tcp = "tcp"
    case Serial = "serial"
    case Bluetooth = "bluetooth"
    case LogReplay = "logReplay"
    case Mock = "mock"
    case AircastCloud = "aircastCloud"
    case Other = ""

    var id: String { rawValue }

    static func from(_ id: String) -> LinkType { allCases.first { $0.id == id && $0 != .Other } ?? .Other }
}

enum LinkEditing: String, CaseIterable {
    case HostAndPort = "hostAndPort"
    case PortOnly = "portOnly"
    case Serial = "serial"
    case LogFile = "logFile"
    case Device = "device"
    case None = ""

    var id: String { rawValue }

    static func from(_ id: String) -> LinkEditing { allCases.first { $0.id == id && $0 != .None } ?? .None }
}

struct LinkRow: Equatable {
    var index: Int
    var name: String
    var statusLine: String
    var goneQuiet = false
    var connected: Bool
    var heard: Bool
    var lastError: String
    var errorRemedy = ""
    var editing: LinkEditing = .None
    var host = ""
    var port = 0
    var portName = ""
    var baud = 0
    var framing = SerialFraming()
    var servers: [String] = []
    var autoConnect = false
    var highLatency = false
    var type: LinkType = .Other
    var filename = ""
}

struct AutoLink: Equatable {
    var name: String
    var summary: String
    var heard: Bool
    var type: LinkType = .Other
    var index = 0
}

func autoLinks(_ view: JSON?) -> [AutoLink] {
    (view?["links"].array ?? [])
        .filter { $0.object != nil && $0["dynamic"].bool && $0["connected"].bool }
        .map { AutoLink(name: $0["name"].string, summary: $0["displaySummary"].string, heard: $0["heardVehicle"].bool, type: LinkType.from($0["type"].string), index: $0["index"].int(0)) }
}

func autoLinkStatus(_ link: AutoLink) -> String { link.heard ? "Vehicle" : "Listening" }

func autoLinkSubtitle(_ link: AutoLink) -> String {
    link.type == .Mock ? "Simulated" : [link.summary, "automatic"].filter { !$0.isBlank }.joined(separator: " \u{00b7} ")
}

func linkRows(_ view: JSON?) -> [LinkRow] {
    (view?["configured"].objects ?? []).map { link in
        LinkRow(
            index: link["index"].int(0),
            name: link["name"].string,
            statusLine: link["statusLine"].string,
            goneQuiet: link["goneQuiet"].bool,
            connected: link["connected"].bool,
            heard: link["heardVehicle"].bool,
            lastError: link["lastError"].string,
            errorRemedy: link["errorRemedy"].string,
            editing: LinkEditing.from(link["editing"].string),
            host: link["host"].string,
            port: link["port"].int(0),
            portName: link["portName"].string,
            baud: link["baud"].int(0),
            framing: SerialFraming(dataBits: link["dataBits"].int(8), stopBits: link["stopBits"].int(1), parity: link["parity"].int(0), flowControl: link["flowControl"].int(0)),
            servers: link["hostList"].array.map(\.string),
            autoConnect: link["autoConnect"].bool,
            highLatency: link["highLatency"].bool,
            type: LinkType.from(link["type"].string),
            filename: link["filename"].string
        )
    }
}

func linkIcon(_ type: LinkType) -> Icon {
    switch type {
    case .Serial: .usb
    case .Udp: .wifi
    case .Bluetooth: .bluetooth
    case .Tcp: .lan
    case .AircastCloud: .cloud
    case .LogReplay: .history
    case .Mock: .science
    case .Other: .link
    }
}

private let REPLAY_LINK_FOLDER = "replay-links"

private func replayFolder(_ link: String) -> URL {
    URL.applicationSupportDirectory
        .appendingPathComponent(REPLAY_LINK_FOLDER, isDirectory: true)
        .appendingPathComponent(link.replacingOccurrences(of: "[^A-Za-z0-9._-]", with: "_", options: .regularExpression), isDirectory: true)
}

func stagedReplayLog(_ link: String, _ uri: URL) -> String? {
    let files = FileManager.default
    let shown = uri.lastPathComponent.replacingOccurrences(of: "[/\\\\]", with: "_", options: .regularExpression).ifBlank("replay.tlog")
    let folder = replayFolder(link)
    let incoming = folder.appendingPathComponent(".incoming")
    let staged = folder.appendingPathComponent(shown)
    let scoped = uri.startAccessingSecurityScopedResource()
    defer { if scoped { uri.stopAccessingSecurityScopedResource() } }
    do {
        try files.createDirectory(at: folder, withIntermediateDirectories: true)
        try? files.removeItem(at: incoming)
        try files.copyItem(at: uri, to: incoming)
        try? files.removeItem(at: staged)
        try files.moveItem(at: incoming, to: staged)
        return staged.path
    } catch {
        return nil
    }
}

func pruneReplayFolder(_ link: String, _ keep: String) {
    let files = FileManager.default
    let kept = URL(fileURLWithPath: keep).resolvingSymlinksInPath().path
    ((try? files.contentsOfDirectory(at: replayFolder(link), includingPropertiesForKeys: nil)) ?? [])
        .filter { $0.resolvingSymlinksInPath().path != kept }
        .forEach { try? files.removeItem(at: $0) }
}

func linkIsEditable(_ row: LinkRow) -> Bool { !row.connected && row.editing != .None }

let CREATABLE_LINK_TYPES: [LinkType] = [.Udp, .Tcp, .Serial]
let REPLAY_LINK_NAME = "Log Replay"

struct BluetoothDeviceChoice: Equatable {
    let name: String
    let address: String
}

struct BluetoothState: Equatable {
    let available: Bool
    let scanning: Bool
    let devices: [BluetoothDeviceChoice]
}

func bluetoothState(_ view: JSON?) -> BluetoothState {
    let bluetooth = view?["bluetooth"] ?? .null
    return BluetoothState(
        available: bluetooth["available"].bool,
        scanning: bluetooth["scanning"].bool,
        devices: bluetooth["devices"].objects.map { BluetoothDeviceChoice(name: $0["name"].string, address: $0["address"].string) }
    )
}

func addableLinkTypes(_ view: JSON?) -> [LinkType] {
    guard view?["linkTypeIds"].arrayOrNil != nil else { return CREATABLE_LINK_TYPES }
    let served = linkTypeIds(view)
    let live = CREATABLE_LINK_TYPES.filter(served.contains)
    let bluetooth = [LinkType.Bluetooth].filter { served.contains($0) && bluetoothState(view).available }
    return (live.isEmpty ? CREATABLE_LINK_TYPES : live) + bluetooth + [LinkType.LogReplay, .Mock].filter(served.contains)
}

func linkTypeIds(_ view: JSON?) -> Set<LinkType> {
    Set((view?["linkTypeIds"].arrayOrNil ?? []).map { LinkType.from($0.string) })
}

struct SerialFraming: Equatable {
    var dataBits = 8
    var stopBits = 1
    var parity = 0
    var flowControl = 0
}

func editWrites(
    _ editing: LinkEditing,
    _ name: String,
    _ host: String,
    _ port: Int,
    _ portName: String,
    _ baud: Int,
    _ autoConnect: Bool,
    _ highLatency: Bool,
    framing: SerialFraming = SerialFraming(),
    logFile: String = ""
) -> [(String, Any)] {
    let common: [(String, Any)] = [("name", name), ("autoConnect", autoConnect), ("highLatency", highLatency)]
    let kind: [(String, Any)] = switch editing {
    case .HostAndPort: [("host", host), ("port", port)]
    case .PortOnly: [("localPort", port)]
    case .LogFile: [("filename", logFile)]
    case .Serial: [
        ("portName", portName),
        ("baud", baud),
        ("dataBits", framing.dataBits),
        ("stopBits", framing.stopBits),
        ("parity", framing.parity),
        ("flowControl", framing.flowControl),
    ]
    case .Device, .None: []
    }
    return common + kind
}

func linkTypeLabel(_ type: LinkType) -> String {
    switch type {
    case .Serial: "Serial"
    case .Bluetooth: "Bluetooth"
    case .AircastCloud: "Aircast Cloud"
    case .LogReplay: "Log replay"
    case .Mock: "Simulated"
    default: type.id.uppercased()
    }
}

func autoLinkName(_ type: LinkType, _ host: String, _ port: String) -> String {
    type == .Udp ? "UDP \(port)" : host.isBlank ? type.id.uppercased() : "\(type.id.uppercased()) \(host):\(port)"
}

func uniqueLinkName(_ base: String, _ taken: [String]) -> String {
    ([base] + (2...(taken.count + 2)).map { "\(base) (\($0))" }).first { !taken.contains($0) } ?? base
}

func editedLinkSuggestion(_ editing: LinkEditing, _ host: String, _ port: String, _ portLabel: String) -> String {
    switch editing {
    case .Serial: autoSerialName(portLabel)
    case .HostAndPort: autoLinkName(.Tcp, host, port)
    default: autoLinkName(.Udp, host, port)
    }
}

let DEFAULT_BAUD = 57600

struct SerialPortChoice: Equatable {
    let port: String
    let label: String
}

func serialPortChoices(_ view: JSON?) -> [SerialPortChoice] {
    (view?["serialPorts"].array ?? [])
        .filter { $0.object != nil && !$0["port"].string.isBlank }
        .map { SerialPortChoice(port: $0["port"].string, label: $0["label"].string.ifBlank($0["port"].string)) }
}

func autoSerialName(_ portLabel: String) -> String { portLabel.ifBlank("Serial") }

func portLabel(_ ports: [SerialPortChoice], _ portName: String) -> String { ports.first { $0.port == portName }?.label ?? "" }

func portFor(_ type: LinkType, _ udpDefault: String) -> String { type == .Tcp ? DEFAULT_TCP_PORT : udpDefault }

func serialFormError(_ portName: String, _ baud: Int, _ taken: [String], _ name: String, _ anyPorts: Bool) -> String? {
    if !anyPorts { return "Nothing is plugged in. Connect a radio over USB and it will appear here." }
    if portName.isBlank { return "Pick the port the radio is plugged into." }
    if baud <= 0 { return "Pick a baud rate." }
    return !name.isBlank && taken.contains(name.trimmed) ? "A link with that name already exists." : nil
}

private func portInRange(_ port: Int?) -> Bool { port.map { (1...65535).contains($0) } ?? false }

func udpServer(_ typed: String, _ localPort: String) -> String? {
    let parts = typed.trimmed.components(separatedBy: ":")
    let host = parts[0].trimmed
    let port = Int((parts.count > 1 ? parts[1] : localPort).trimmed)
    guard parts.count <= 2, !host.isEmpty, let port, portInRange(port) else { return nil }
    return "\(host):\(port)"
}

func withServer(_ servers: [String], _ typed: String, _ localPort: String) -> [String] {
    udpServer(typed, localPort).flatMap { servers.contains($0) ? nil : servers + [$0] } ?? servers
}

private func parsedPort(_ type: LinkType, _ port: String, _ udpDefault: String) -> Int? {
    Int(port.isBlank && type == .Udp ? udpDefault : port)
}

func linkFormError(_ type: LinkType, _ host: String, _ port: String, udpDefault: String = DEFAULT_PORT) -> String? {
    let parsed = parsedPort(type, port, udpDefault)
    if type == .Udp && !portInRange(parsed) { return "Enter a port between 1 and 65535, or leave it blank for \(udpDefault)" }
    if !portInRange(parsed) { return "Port must be a number between 1 and 65535." }
    return type == .Tcp && host.isBlank ? "A TCP link needs the address of the device to call." : nil
}

func linkFormErrorField(_ type: LinkType, _ host: String, _ port: String, udpDefault: String = DEFAULT_PORT) -> String? {
    !portInRange(parsedPort(type, port, udpDefault)) ? "port" : type == .Tcp && host.isBlank ? "host" : nil
}

private let CONNECTED_STATUS = "Connected"

private struct LinkIconBadge: View {
    let type: LinkType
    let lit: Bool
    @Environment(\.theme) private var theme

    var body: some View {
        Image(linkIcon(type))
            .font(.system(size: 18))
            .foregroundStyle(lit ? theme.aircast.success : theme.colors.onSecondaryContainer)
            .frame(width: 40, height: 40)
            .background(lit ? theme.aircast.successContainer : theme.colors.secondaryContainer, in: Circle())
    }
}

private struct AutoLinkItem: View {
    let link: AutoLink
    var onStop: (() -> Void)? = nil
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: 16) {
            LinkIconBadge(type: link.type, lit: link.heard)
            VStack(alignment: .leading, spacing: 2) {
                Text(link.name).font(.bodyLarge).foregroundStyle(theme.colors.onSurface).lineLimit(1)
                Text(autoLinkSubtitle(link)).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            Text(autoLinkStatus(link)).font(.labelMedium).foregroundStyle(link.heard ? theme.aircast.success : theme.colors.onSurfaceVariant)
            if let onStop {
                Button("Stop", action: onStop).buttonStyle(.borderless)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
        .frame(maxWidth: .infinity, minHeight: 72)
    }
}

private struct LinkRowItem: View {
    let row: LinkRow
    let onConnect: () -> Void
    let onDisconnect: () -> Void
    let onRemove: () -> Void
    let onEdit: () -> Void
    @Environment(\.theme) private var theme

    private var fixByEditing: Bool { row.errorRemedy == REMEDY_EDIT_ADDRESS && linkIsEditable(row) }

    var body: some View {
        Menu {
            if fixByEditing {
                edit
                primary
            } else {
                primary
                edit
            }
            Button("Delete link", role: .destructive, action: onRemove)
        } label: {
            content
        }
        .menuOrder(.fixed)
        .menuStyle(.button)
        .buttonStyle(.plain)
        .accessibilityLabel("More actions for \(row.name)")
    }

    private var primary: some View {
        Button(primaryLinkAction(row.connected, fixByEditing)) { row.connected ? onDisconnect() : onConnect() }
    }

    private var edit: some View {
        Button("Edit link", action: onEdit).disabled(!linkIsEditable(row))
    }

    private var content: some View {
        HStack(spacing: 16) {
            LinkIconBadge(type: row.type, lit: row.connected)
            VStack(alignment: .leading, spacing: 2) {
                Text(row.name).font(.bodyLarge).foregroundStyle(theme.colors.onSurface).lineLimit(1)
                if !(row.connected && row.statusLine == CONNECTED_STATUS) {
                    Text(row.statusLine).font(.bodyMedium).foregroundStyle(row.goneQuiet ? theme.colors.error : theme.colors.onSurfaceVariant)
                }
                if !row.lastError.isBlank {
                    Text(row.lastError).font(.bodySmall).foregroundStyle(theme.colors.error)
                    if let advice = remedyText(row.errorRemedy) {
                        Text(advice).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                }
            }
            .multilineTextAlignment(.leading)
            .frame(maxWidth: .infinity, alignment: .leading)
            if row.connected {
                Text(CONNECTED_STATUS).font(.labelLarge).foregroundStyle(theme.aircast.success)
            }
            Image(.chevronRight).foregroundStyle(theme.colors.onSurfaceVariant)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
        .frame(maxWidth: .infinity, minHeight: 72)
        .contentShape(Rectangle())
    }
}

func primaryLinkAction(_ connected: Bool, _ fixByEditing: Bool) -> String {
    connected ? "Disconnect" : fixByEditing ? "Try again" : "Connect"
}

private struct AdvancedDisclosure: View {
    let open: Bool
    let onToggle: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onToggle) {
            HStack {
                Text("Advanced").font(.titleSmall).foregroundStyle(theme.colors.onSurface)
                Spacer()
                Image(open ? .arrowUp : .arrowDropDown)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .accessibilityLabel(open ? "Hide advanced options" : "Show advanced options")
            }
            .frame(maxWidth: .infinity, minHeight: 48)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

private struct LinkFlagSwitches: View {
    let autoConnect: Bool
    let highLatency: Bool
    let onAutoConnect: (Bool) -> Void
    let onHighLatency: (Bool) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        flag(LINK_FLAG_TEXT[0], autoConnect, onAutoConnect)
        flag(LINK_FLAG_TEXT[1], highLatency, onHighLatency)
    }

    private func flag(_ text: (String, String), _ checked: Bool, _ onChange: @escaping (Bool) -> Void) -> some View {
        Toggle(isOn: Binding(get: { checked }, set: onChange)) {
            VStack(alignment: .leading, spacing: 2) {
                Text(text.0)
                Text(text.1).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
    }
}

let LINK_FLAG_TEXT: [(String, String)] = [
    ("Connect on start", "Open this link when the app launches."),
    ("High latency", "Tune the link for satellite or cellular round trips."),
]

func linkFlagWrites(_ index: Int, _ autoConnect: Bool, _ highLatency: Bool) -> [(String, Bool)] {
    [("\(LINKS_PATH).\(index).autoConnect", autoConnect), ("\(LINKS_PATH).\(index).highLatency", highLatency)]
}

func framingWrites(_ index: Int, _ framing: SerialFraming) -> [(String, Int)] {
    [
        ("dataBits", framing.dataBits),
        ("stopBits", framing.stopBits),
        ("parity", framing.parity),
        ("flowControl", framing.flowControl),
    ].map { field, value in ("\(LINKS_PATH).\(index).\(field)", value) }
}

private func writeNewLinkFraming(_ name: String, _ framing: SerialFraming) -> Bool {
    if framing == SerialFraming() { return true }
    guard let row = currentRows().first(where: { $0.name == name }) else { return false }
    framingWrites(row.index, framing).forEach { path, value in Qgc.set(path, value) }
    LinkCommands.commitConfigurations()
    return true
}

@discardableResult
private func addServers(_ name: String, _ servers: [String]) -> Bool {
    if servers.isEmpty { return true }
    guard let row = currentRows().first(where: { $0.name == name }) else { return false }
    servers.forEach { Qgc.invoke("\(LINKS_PATH).\(row.index).addHost", $0) }
    LinkCommands.commitConfigurations()
    return true
}

private func writeNewLinkFlags(_ name: String, _ autoConnect: Bool, _ highLatency: Bool) {
    guard autoConnect || highLatency, let row = currentRows().first(where: { $0.name == name }) else { return }
    linkFlagWrites(row.index, autoConnect, highLatency).forEach { path, value in Qgc.set(path, value) }
    LinkCommands.commitConfigurations()
}

struct LinkEdit: Equatable {
    var name: String
    var host: String
    var port: String
    var portName: String
    var baud: Int
    var autoConnect: Bool
    var highLatency: Bool
    var framing: SerialFraming
    var logFile: String
}

func linkEdit(_ row: LinkRow) -> LinkEdit {
    LinkEdit(
        name: row.name,
        host: row.host,
        port: String(row.port),
        portName: row.portName,
        baud: row.baud > 0 ? row.baud : DEFAULT_BAUD,
        autoConnect: row.autoConnect,
        highLatency: row.highLatency,
        framing: row.framing,
        logFile: row.filename
    )
}

func editShowsAdvanced(_ row: LinkRow) -> Bool {
    row.autoConnect || row.highLatency || !row.servers.isEmpty || row.framing != SerialFraming()
}

func editedPort(_ editing: LinkEditing, _ port: String, _ udpDefault: String) -> Int {
    Int(port.isBlank && editing == .PortOnly ? udpDefault : port) ?? 0
}

func editError(_ editing: LinkEditing, _ edit: LinkEdit, _ udpDefault: String) -> String? {
    switch editing {
    case .Serial, .Device, .None: nil
    case .LogFile: edit.logFile.isBlank ? "Choose a log file." : nil
    case .PortOnly: linkFormError(.Udp, "", edit.port, udpDefault: udpDefault)
    case .HostAndPort: portInRange(editedPort(editing, edit.port, udpDefault)) ? nil : "Port must be a number between 1 and 65535."
    }
}

func editSuggestion(_ row: LinkRow, _ edit: LinkEdit, _ device: BluetoothDeviceChoice?, _ ports: [SerialPortChoice]) -> String {
    row.editing == .Device ? device?.name ?? row.name : editedLinkSuggestion(row.editing, edit.host, edit.port, portLabel(ports, edit.portName))
}

let EDIT_WHILE_CONNECTED = "Disconnect this link to change it."

private func applyEdit(_ index: Int, _ writes: [(String, Any)], _ device: BluetoothDeviceChoice?) {
    writes.forEach { field, value in Qgc.set("\(LINKS_PATH).\(index).\(field)", value) }
    if let device { Qgc.invoke("\(LINKS_PATH).\(index).setDeviceByAddress", device.address) }
    LinkCommands.commitConfigurations()
    LinkCommands.connect("@\(LINKS_PATH).\(index)")
}

private func udpPortOrDefault(_ value: JSON) -> String {
    guard case .number = value, let port = value.int, portInRange(port) else { return DEFAULT_PORT }
    return String(port)
}

private struct LinkTextField: View {
    let label: String
    let value: String
    var placeholder = ""
    var error: String? = nil
    var enabled = true
    var keyboard: UIKeyboardType = .default
    var onSubmit: () -> Void = {}
    let onChange: (String) -> Void
    @Environment(\.theme) private var theme
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label).font(.labelMedium).foregroundStyle(error == nil ? theme.colors.onSurfaceVariant : theme.colors.error)
            TextField(placeholder, text: Binding(get: { value }, set: onChange))
                .keyboardType(keyboard)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .textFieldStyle(.roundedBorder)
                .submitLabel(.done)
                .onSubmit(onSubmit)
                .focused($focused)
                .toolbar {
                    if focused && KEYBOARDS_WITHOUT_RETURN.contains(keyboard) {
                        ToolbarItemGroup(placement: .keyboard) {
                            Spacer()
                            Button("Done") { focused = false }
                        }
                    }
                }
                .disabled(!enabled)
            if let error {
                Text(error).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct LinkPortField: View {
    let label: String
    let value: String
    let error: String?
    let enabled: Bool
    let onChange: (String) -> Void

    var body: some View {
        LinkTextField(label: label, value: value, error: error, enabled: enabled, keyboard: .numberPad, onChange: onChange)
    }
}

private struct LinkHostField: View {
    let value: String
    let error: String?
    let enabled: Bool
    let onChange: (String) -> Void

    var body: some View {
        LinkTextField(label: "Host address", value: value, placeholder: "Example: 192.168.1.10", error: error, enabled: enabled, onChange: onChange)
    }
}

private struct EditLinkFields: View {
    let editing: LinkEditing
    let edit: LinkEdit
    let enabled: Bool
    let onPickLog: () -> Void
    let onEdit: (LinkEdit) -> Void

    var body: some View {
        switch editing {
        case .LogFile:
            Text((edit.logFile.components(separatedBy: "/").last ?? "").ifBlank("No log file chosen")).font(.bodyLarge)
            Button(action: onPickLog) { Text("Choose another log file").frame(maxWidth: .infinity) }
                .buttonStyle(.bordered)
                .disabled(!enabled)
        case .HostAndPort:
            LinkHostField(value: edit.host, error: nil, enabled: enabled) { host in onEdit(withChanges(edit) { $0.host = host }) }
            LinkPortField(label: "Port", value: edit.port, error: nil, enabled: enabled) { port in onEdit(withChanges(edit) { $0.port = port }) }
        case .PortOnly, .None:
            LinkPortField(label: editing == .PortOnly ? "Listening port" : "Port", value: edit.port, error: nil, enabled: enabled) { port in onEdit(withChanges(edit) { $0.port = port }) }
        case .Serial, .Device:
            EmptyView()
        }
    }
}

private struct PageFrame<Content: View>: View {
    let title: String
    let backLabel: String
    let onBack: () -> Void
    let action: String
    let actionEnabled: Bool
    let onAction: () -> Void
    @ViewBuilder let content: () -> Content
    @Environment(\.LocalPageHeading) private var pageHeading

    var body: some View {
        VStack(spacing: 0) {
            OverridePageHeading(title: title, onBack: onBack)
            if pageHeading == nil {
                PageTopBar(title: title, backLabel: backLabel, onBack: onBack)
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 12, content: content)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            Button(action: onAction) { Text(action).frame(maxWidth: .infinity, minHeight: 52) }
                .buttonStyle(.filled)
                .disabled(!actionEnabled)
                .padding(16)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

private struct EditLinkPage: View {
    let row: LinkRow
    let onDismiss: () -> Void
    let onSaved: () -> Void
    @State private var edit: LinkEdit
    @State private var advanced: Bool
    @State private var error: String?
    @State private var picking = false
    @QgcPath(LINKS_VIEW) private var linksJson
    @QgcValue(UDP_LISTEN_PORT) private var udpListen
    @Environment(\.theme) private var theme

    init(row: LinkRow, onDismiss: @escaping () -> Void, onSaved: @escaping () -> Void) {
        self.row = row
        self.onDismiss = onDismiss
        self.onSaved = onSaved
        _edit = State(initialValue: linkEdit(row))
        _advanced = State(initialValue: editShowsAdvanced(row))
    }

    private var udpDefault: String { udpPortOrDefault(udpListen) }
    private var ports: [SerialPortChoice] { serialPortChoices(linksJson) }
    private var editable: Bool { linkRows(linksJson).first { $0.index == row.index }?.connected != true }

    var body: some View {
        PageFrame(title: "Edit \(row.name)", backLabel: "Back to links", onBack: cancel, action: "Save and connect", actionEnabled: editable, onAction: save) {
            if !linkKind(row.type).about.isBlank {
                Text(linkKind(row.type).about).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            if !editable {
                Text(EDIT_WHILE_CONNECTED).font(.bodyMedium).foregroundStyle(theme.colors.error)
            }
            EditLinkFields(editing: row.editing, edit: edit, enabled: editable, onPickLog: { picking = true }) { edit = $0 }
            AdvancedDisclosure(open: advanced) { advanced.toggle() }
            if advanced {
                LinkTextField(label: "Name", value: edit.name, enabled: editable) { name in edit.name = name }
                if row.editing == .PortOnly {
                    UdpServers(index: row.index, initial: row.servers)
                }
                LinkFlagSwitches(autoConnect: edit.autoConnect, highLatency: edit.highLatency, onAutoConnect: { edit.autoConnect = $0 }, onHighLatency: { edit.highLatency = $0 })
            }
            if let error {
                Text(error).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
        }
        .fileImporter(isPresented: $picking, allowedContentTypes: [.item]) { result in
            guard case .success(let chosen) = result else { return }
            let link = row.name
            Task { @MainActor in
                let staged = await offMain { stagedReplayLog(link, chosen) }
                if let staged { edit.logFile = staged } else { error = "That file could not be read." }
            }
        }
    }

    private func cancel() {
        if row.editing == .LogFile {
            let link = row.name, kept = row.filename
            offMain { pruneReplayFolder(link, kept) }
        }
        onDismiss()
    }

    private func save() {
        error = editError(row.editing, edit, udpDefault)
        guard error == nil else { return }
        let row = row, edit = edit, udpDefault = udpDefault, ports = ports
        Task { @MainActor in
            let rows = await offMain { currentRows() }
            guard rows.first(where: { $0.index == row.index })?.connected == false else {
                error = EDIT_WHILE_CONNECTED
                return
            }
            let resolved = edit.name.trimmed.ifBlank(uniqueLinkName(editSuggestion(row, edit, nil, ports), rows.filter { $0.index != row.index }.map(\.name)))
            let writes = editWrites(row.editing, resolved, edit.host, editedPort(row.editing, edit.port, udpDefault), edit.portName, edit.baud, edit.autoConnect, edit.highLatency, framing: edit.framing, logFile: edit.logFile)
            await offMain { applyEdit(row.index, writes, nil) }
            if row.editing == .LogFile {
                await offMain { pruneReplayFolder(row.name, edit.logFile) }
            }
            onSaved()
        }
    }
}

struct LinkKind: Equatable {
    let type: LinkType
    let title: String
    let detail: String
    let about: String
}

let PILOT_LINK_ORDER: [LinkType] = [.Udp, .Serial, .Bluetooth, .AircastCloud, .Tcp]
let TOOL_LINK_ORDER: [LinkType] = [.LogReplay, .Mock]

func linkKind(_ type: LinkType) -> LinkKind {
    switch type {
    case .Udp: LinkKind(type: type, title: "Wi-Fi or network", detail: "Wi-Fi or Ethernet (UDP)", about: "Listens on a port for telemetry from the aircraft or its radio.")
    case .Serial: LinkKind(type: type, title: "USB radio", detail: "Telemetry radio on USB", about: "A radio plugged into this device over USB.")
    case .Bluetooth: LinkKind(type: type, title: "Bluetooth radio", detail: "Paired telemetry radio", about: "A radio paired with or near this device over Bluetooth.")
    case .AircastCloud: LinkKind(type: type, title: "Aircast Cloud", detail: "Backup link via your account", about: "A backup link to the aircraft through your Aircast account.")
    case .Tcp: LinkKind(type: type, title: "Network server (TCP)", detail: "A listening ground station", about: "Calls out to a device that is listening, such as a ground station or bridge.")
    case .LogReplay: LinkKind(type: type, title: "Replay a flight log", detail: "Play back a saved log", about: "Plays back a saved telemetry log as if the vehicle were connected.")
    case .Mock: LinkKind(type: type, title: "Simulated vehicle", detail: "Try the app without hardware", about: "A simulated vehicle for trying the app without hardware. It is not saved, so it ends when the app restarts.")
    case .Other: LinkKind(type: type, title: linkTypeLabel(type), detail: "", about: "")
    }
}

func pilotLinkKinds(_ offered: [LinkType], _ radioPlugged: Bool) -> [LinkType] {
    let listed = PILOT_LINK_ORDER.filter(offered.contains)
    return radioPlugged && listed.contains(.Serial) ? [.Serial] + listed.filter { $0 != .Serial } : listed
}

func toolLinkKinds(_ offered: [LinkType]) -> [LinkType] { TOOL_LINK_ORDER.filter(offered.contains) }

struct LinkDraft: Equatable {
    var type: LinkType
    var name = ""
    var host = ""
    var port = ""
    var servers: [String] = []
    var portName = ""
    var baud = DEFAULT_BAUD
    var framing = SerialFraming()
    var autoConnect = false
    var highLatency = false
    var device: BluetoothDeviceChoice? = nil
    var replayLog = ""
    var apiBase = ""
    var deviceId = ""
    var mock = MockLinkChoices()
}

extension LinkDraft {
    func shownPort(_ udpDefault: String) -> String { port.ifBlank(portFor(type, udpDefault)) }

    func withPluggedPort(_ ports: [SerialPortChoice]) -> LinkDraft {
        withChanges(self) { $0.portName = portName.ifBlank(ports.count == 1 ? ports[0].port : "") }
    }
}

func suggestedLinkName(_ draft: LinkDraft, _ ports: [SerialPortChoice], _ udpDefault: String) -> String {
    switch draft.type {
    case .Serial: autoSerialName(portLabel(ports, draft.portName))
    case .LogReplay: REPLAY_LINK_NAME
    case .AircastCloud: AIRCAST_CLOUD_NAME
    case .Bluetooth: draft.device?.name ?? ""
    default: autoLinkName(draft.type, draft.host, draft.shownPort(udpDefault))
    }
}

func draftError(_ draft: LinkDraft, _ udpDefault: String, _ taken: [String], _ anyPorts: Bool) -> String? {
    switch draft.type {
    case .Mock: nil
    case .AircastCloud: cloudApiBaseValid(draft.apiBase) && cloudDeviceValid(draft.deviceId) ? nil : ""
    case .Bluetooth: draft.device == nil ? "Pick a Bluetooth device." : nil
    case .LogReplay: draft.replayLog.isBlank ? "Choose a log file to replay." : nil
    case .Serial: serialFormError(draft.portName, draft.baud, taken, draft.name, anyPorts)
    default: linkFormError(draft.type, draft.host, draft.shownPort(udpDefault), udpDefault: udpDefault)
    }
}

enum AddOutcome { case Connected, SavedNotConnected, Failed }

func addOutcome(_ created: Bool, _ saved: Bool) -> AddOutcome {
    created ? .Connected : saved ? .SavedNotConnected : .Failed
}

func addFailure(_ type: LinkType) -> String {
    type == .Mock ? "Could not start the simulated vehicle." : "Could not add that link. The name may already be in use."
}

private func createLink(_ draft: LinkDraft, _ name: String, _ port: Int, _ staged: String?) -> Bool {
    switch draft.type {
    case .Mock: LinkCommands.startMock(mockLinkArguments(draft.mock))
    case .AircastCloud: createCloudLink(draft, name)
    case .LogReplay: staged.map { LinkCommands.createLogReplay(name, log: $0) && connectNamed(name) } ?? false
    case .Bluetooth: draft.device.map { LinkCommands.createBluetooth(name, deviceName: $0.name, address: $0.address) } ?? false
    case .Serial: LinkCommands.createSerial(name, port: draft.portName, baud: draft.baud) && writeNewLinkFraming(name, draft.framing) && connectNamed(name)
    default: Qgc.invokeResult("links.createAndConnectLink", draft.type.id, name, draft.type == .Udp ? "" : draft.host, port) == .bool(true)
    }
}

private func createCloudLink(_ draft: LinkDraft, _ name: String) -> Bool {
    AccountCommands.setApiBase(draft.apiBase)
    return LinkCommands.createAircastCloud(name, apiBase: draft.apiBase, deviceId: draft.deviceId)
}

private func addLink(_ draft: LinkDraft, _ name: String, _ udpDefault: String, _ pickedLog: URL?) -> AddOutcome {
    let staged = draft.type == .LogReplay ? pickedLog.flatMap { stagedReplayLog(name, $0) } : nil
    let created = createLink(draft, name, Int(draft.shownPort(udpDefault)) ?? 0, staged)
    let saved = draft.type != .Mock && currentRows().contains { $0.name == name }
    let outcome = addOutcome(created, saved)
    if outcome == .Failed, let staged { try? FileManager.default.removeItem(atPath: staged) }
    if saved {
        addServers(name, draft.servers)
        writeNewLinkFlags(name, draft.autoConnect, draft.highLatency)
    }
    return outcome
}

private struct AddLinkPage: View {
    let onDismiss: () -> Void
    let onAdded: () -> Void
    @State private var type: LinkType?
    @State private var cloudOffered = false
    @QgcPath(LINKS_VIEW) private var linksJson
    @Environment(\.LocalPageHeading) private var pageHeading

    private var choices: [LinkType] {
        (addableLinkTypes(linksJson) + [LinkType.AircastCloud].filter { cloudOffered && linkTypeIds(linksJson).contains($0) })
            .filter { !HOST_CANNOT_OPEN.contains($0) }
    }

    var body: some View {
        Group {
            if let chosen = type {
                AddLinkDetails(type: chosen, onBack: { type = nil }, onAdded: { if type == chosen { onAdded() } })
            } else {
                picker
            }
        }
        .task { cloudOffered = await offMain { accountState(AccountCommands.state()) != nil } }
    }

    private var picker: some View {
        VStack(spacing: 0) {
            OverridePageHeading(title: "Add link", onBack: onDismiss)
            if pageHeading == nil {
                PageTopBar(title: "Add link", backLabel: "Back to links", onBack: onDismiss)
            }
            ScrollView {
                VStack(spacing: 0) {
                    SectionHeader(text: "Connect over")
                    ForEach(pilotLinkKinds(choices, !serialPortChoices(linksJson).isEmpty).map(linkKind), id: \.type) { row($0) }
                    let tools = toolLinkKinds(choices)
                    if !tools.isEmpty {
                        SectionHeader(text: "Other")
                        ForEach(tools.map(linkKind), id: \.type) { row($0) }
                    }
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func row(_ kind: LinkKind) -> some View {
        SetupRow(title: kind.title, onClick: { type = kind.type }, icon: linkIcon(kind.type), subtitle: kind.detail)
    }
}

private struct NewLinkFields: View {
    let draft: LinkDraft
    let hostError: String?
    let portError: String?
    let cloudErrors: Bool
    let onPickLog: () -> Void
    let onDraft: (LinkDraft) -> Void

    var body: some View {
        switch draft.type {
        case .Mock:
            MockLinkFields(choices: draft.mock) { mock in onDraft(withChanges(draft) { $0.mock = mock }) }
        case .LogReplay:
            Button(action: onPickLog) {
                Text(draft.replayLog.isBlank ? "Choose a log file" : URL(string: draft.replayLog)?.lastPathComponent ?? "Choose a log file")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.bordered)
        case .AircastCloud:
            AircastCloudFields(
                apiBase: draft.apiBase,
                deviceId: draft.deviceId,
                showErrors: cloudErrors,
                onApiBase: { base in onDraft(withChanges(draft) { $0.apiBase = base }) },
                onDeviceId: { device in onDraft(withChanges(draft) { $0.deviceId = device }) }
            )
        case .Serial, .Bluetooth:
            EmptyView()
        case .Udp:
            LinkPortField(label: "Listening port", value: draft.port, error: portError, enabled: true) { port in onDraft(withChanges(draft) { $0.port = port }) }
        case .Tcp, .Other:
            LinkHostField(value: draft.host, error: hostError, enabled: true) { host in onDraft(withChanges(draft) { $0.host = host }) }
            LinkPortField(label: "Port", value: draft.port, error: portError, enabled: true) { port in onDraft(withChanges(draft) { $0.port = port }) }
        }
    }
}

private struct NewLinkAdvanced: View {
    let draft: LinkDraft
    let suggestedName: String
    let udpDefault: String
    let onError: (String?) -> Void
    let onDraft: (LinkDraft) -> Void

    var body: some View {
        LinkTextField(label: "Name", value: draft.name, placeholder: suggestedName) { name in onDraft(withChanges(draft) { $0.name = name }) }
        if draft.type == .Udp {
            ServerList(
                servers: draft.servers,
                onAdd: { typed in
                    onError(udpServer(typed, draft.shownPort(udpDefault)) == nil ? "Enter a server as an address, or address:port." : nil)
                    onDraft(withChanges(draft) { $0.servers = withServer(draft.servers, typed, draft.shownPort(udpDefault)) })
                },
                onRemove: { gone in onDraft(withChanges(draft) { $0.servers = draft.servers.filter { $0 != gone } }) }
            )
        }
        LinkFlagSwitches(
            autoConnect: draft.autoConnect,
            highLatency: draft.highLatency,
            onAutoConnect: { on in onDraft(withChanges(draft) { $0.autoConnect = on }) },
            onHighLatency: { on in onDraft(withChanges(draft) { $0.highLatency = on }) }
        )
    }
}

private struct AddLinkDetails: View {
    let type: LinkType
    let onBack: () -> Void
    let onAdded: () -> Void
    @State private var typed: LinkDraft
    @State private var pickedLog: URL?
    @State private var advanced = false
    @State private var error: String?
    @State private var busy = false
    @State private var cloudErrors = false
    @State private var picking = false
    @QgcPath(LINKS_VIEW) private var linksJson
    @QgcValue(UDP_LISTEN_PORT) private var udpListen
    @Environment(\.theme) private var theme

    init(type: LinkType, onBack: @escaping () -> Void, onAdded: @escaping () -> Void) {
        self.type = type
        self.onBack = onBack
        self.onAdded = onAdded
        _typed = State(initialValue: LinkDraft(type: type, port: portFor(type, DEFAULT_PORT)))
    }

    private var ports: [SerialPortChoice] { serialPortChoices(linksJson) }
    private var taken: [String] { linkRows(linksJson).map(\.name) }
    private var udpDefault: String { udpPortOrDefault(udpListen) }
    private var draft: LinkDraft { typed.withPluggedPort(ports) }
    private var suggestedName: String { suggestedLinkName(draft, ports, udpDefault) }

    private var fieldError: String? {
        let port = draft.shownPort(udpDefault)
        guard let error, type == .Udp || type == .Tcp, error == linkFormError(type, draft.host, port, udpDefault: udpDefault) else { return nil }
        return linkFormErrorField(type, draft.host, port, udpDefault: udpDefault)
    }

    var body: some View {
        PageFrame(title: linkKind(type).title, backLabel: "Back to link types", onBack: onBack, action: busy ? "Connecting…" : type == .Mock ? "Start" : "Connect", actionEnabled: !busy, onAction: connect) {
            Text(linkKind(type).about).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            NewLinkFields(
                draft: draft,
                hostError: fieldError == "host" ? error : nil,
                portError: fieldError == "port" ? error : nil,
                cloudErrors: cloudErrors,
                onPickLog: { picking = true },
                onDraft: change
            )
            if type != .Mock {
                AdvancedDisclosure(open: advanced) { advanced.toggle() }
                if advanced {
                    NewLinkAdvanced(draft: draft, suggestedName: suggestedName, udpDefault: udpDefault, onError: { error = $0 }, onDraft: change)
                }
            }
            if fieldError == nil, let error {
                Text(error).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
        }
        .fileImporter(isPresented: $picking, allowedContentTypes: [.item]) { result in
            guard case .success(let chosen) = result else { return }
            pickedLog = chosen
            typed.replayLog = chosen.absoluteString
        }
        .onChange(of: udpDefault, initial: true) { old, new in
            if typed.type == .Udp && [old, DEFAULT_PORT].contains(typed.port) { typed.port = new }
        }
    }

    private func change(_ next: LinkDraft) {
        let clearsError = fieldError != nil
        typed = next
        if clearsError { error = nil }
    }

    private func connect() {
        cloudErrors = type == .AircastCloud
        let invalid = draftError(draft, udpDefault, taken, !ports.isEmpty)
        error = invalid.flatMap { $0.isBlank ? nil : $0 }
        guard invalid == nil else { return }
        busy = true
        let draft = draft, udpDefault = udpDefault, picked = pickedLog, type = type
        let name = draft.name.trimmed.ifBlank(uniqueLinkName(suggestedName, taken))
        Task { @MainActor in
            let outcome = await offMain { addLink(draft, name, udpDefault, picked) }
            busy = false
            if outcome == .Failed { error = addFailure(type) } else { onAdded() }
        }
    }
}

private let LINK_SETTLE_MS = 4000

func linkFailure(_ action: String, _ done: Bool) -> String? { done ? nil : "Could not \(action) that link." }

private func currentRows() -> [LinkRow] { linkRows(Qgc.get(LINKS_VIEW)) }

private func connectNamed(_ name: String) -> Bool {
    guard let row = currentRows().first(where: { $0.name == name }) else { return false }
    LinkCommands.connect("@\(LINKS_PATH).\(row.index)")
    return true
}

private func settles(_ dispatched: Bool, _ settled: @escaping @Sendable () -> Bool) async -> Bool {
    guard dispatched else { return false }
    let deadline = ContinuousClock.now.advanced(by: .milliseconds(LINK_SETTLE_MS))
    while ContinuousClock.now < deadline {
        if await offMain({ settled() }) { return true }
        try? await Task.sleep(for: .milliseconds(150))
    }
    return false
}

private func rowConnected(_ index: Int) -> Bool? { currentRows().first { $0.index == index }?.connected }

struct LinksScreen<Footer: View>: View {
    @ViewBuilder let footer: () -> Footer
    @QgcPath(LINKS_VIEW) private var view
    @HasVehicle private var hasVehicle
    @Environment(AppNavigationState.self) private var navigation
    @Environment(\.theme) private var theme
    @State private var notice: String?
    @State private var adding = false
    @State private var advanced = false
    @State private var confirmingDisconnect: LinkRow?
    @State private var confirmingRemove: LinkRow?
    @State private var editing: LinkRow?

    var body: some View {
        Group {
            if adding {
                AddLinkPage(onDismiss: { adding = false }, onAdded: { adding = false })
            } else if let row = editing {
                EditLinkPage(row: row, onDismiss: { editing = nil }, onSaved: { if editing == row { editing = nil } })
            } else {
                list
            }
        }
        .onChange(of: navigation.addLinkRequested, initial: true) { _, requested in
            guard requested else { return }
            adding = true
            navigation.addLinkRequested = false
        }
    }

    private var list: some View {
        let rows = linkRows(view)
        let auto = autoLinks(view)
        return VStack(alignment: .leading, spacing: 0) {
            if let notice {
                Text(notice).font(.bodySmall).foregroundStyle(theme.colors.error).padding(.horizontal, 20).padding(.vertical, 8)
            }
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    if rows.isEmpty && auto.isEmpty {
                        FootNote(text: "No links saved. Add a link for Wi-Fi or a network connection.")
                    }
                    section("Connected", rows.filter(\.connected), auto)
                    section("Saved", rows.filter { !$0.connected }, [])
                    Button { adding = true } label: {
                        HStack(spacing: 8) {
                            Image(.add)
                            Text("Add link\u{2026}")
                        }
                    }
                    .buttonStyle(.filled)
                    .padding(.horizontal, 20)
                    .padding(.vertical, 8)
                    AdvancedToggle(open: advanced) { advanced.toggle() }
                    if advanced { footer() }
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .alert(
            "Disconnect \(confirmingDisconnect?.name ?? "")?",
            isPresented: presented($confirmingDisconnect),
            presenting: confirmingDisconnect
        ) { row in
            Button("Disconnect") { disconnect(row) }
            Button("Keep connected", role: .cancel) {}
        } message: { _ in
            Text("A vehicle is connected on this link. Disconnecting stops telemetry and you will not be able to command it until it reconnects.")
        }
        .alert(
            "Remove \(confirmingRemove?.name ?? "")?",
            isPresented: presented($confirmingRemove),
            presenting: confirmingRemove
        ) { row in
            Button("Remove", role: .destructive) {
                let name = row.name, configuration = "@\(LINKS_PATH).\(row.index)"
                attempt("remove", { !currentRows().contains { $0.name == name } }) { LinkCommands.remove(configuration) }
            }
            Button("Cancel", role: .cancel) {}
        } message: { row in
            Text(row.connected ? "This link is connected. Removing it disconnects it first and forgets the settings." : "Aircast will forget this link and its settings.")
        }
    }

    @ViewBuilder
    private func section(_ title: String, _ group: [LinkRow], _ auto: [AutoLink]) -> some View {
        if !group.isEmpty || !auto.isEmpty {
            SectionHeader(text: title)
        }
        ForEach(group, id: \.name) { row in
            LinkRowItem(
                row: row,
                onConnect: {
                    let index = row.index
                    attempt("connect", { rowConnected(index) == true }) { LinkCommands.connect("@\(LINKS_PATH).\(index)") }
                },
                onDisconnect: {
                    if hasVehicle { confirmingDisconnect = row } else { disconnect(row) }
                },
                onRemove: { confirmingRemove = row },
                onEdit: { editing = row }
            )
        }
        ForEach(auto, id: \.name) { link in
            AutoLinkItem(link: link, onStop: link.type == .Mock ? { stop(link) } : nil)
        }
    }

    private func disconnect(_ row: LinkRow) {
        let index = row.index
        attempt("disconnect", { rowConnected(index) != true }) { Qgc.invoke("\(LINKS_PATH).\(index).link.disconnect") }
    }

    private func stop(_ link: AutoLink) {
        let name = link.name, index = link.index
        attempt("stop", { !autoLinks(Qgc.get(LINKS_VIEW)).contains { $0.name == name } }) { Qgc.invoke("\(LINKS_PATH).\(index).link.disconnect") }
    }

    private func attempt(_ action: String, _ settled: @escaping @Sendable () -> Bool, _ call: @escaping @Sendable () -> Bool) {
        Task { @MainActor in
            let dispatched = await offMain { call() }
            let done = await settles(dispatched, settled)
            notice = linkFailure(action, done)
        }
    }
}

extension LinksScreen where Footer == EmptyView {
    init() { self.init(footer: { EmptyView() }) }
}

let REMEDY_EDIT_ADDRESS = "editAddress"

func remedyText(_ remedy: String) -> String? {
    remedy == REMEDY_EDIT_ADDRESS ? "Nothing is listening at that address. Retrying will not help until it is changed." : nil
}

private struct UdpServers: View {
    let index: Int
    let initial: [String]
    @State private var servers: [String]

    init(index: Int, initial: [String]) {
        self.index = index
        self.initial = initial
        _servers = State(initialValue: initial)
    }

    var body: some View {
        ServerList(servers: servers, onAdd: { change("addHost", $0) }, onRemove: { change("removeHost", $0) })
    }

    private func change(_ action: String, _ host: String) {
        let index = index, current = servers
        Task { @MainActor in
            servers = await offMain {
                Qgc.invoke("\(LINKS_PATH).\(index).\(action)", host)
                return currentRows().first { $0.index == index }?.servers ?? current
            }
        }
    }
}

private struct ServerList: View {
    let servers: [String]
    let onAdd: (String) -> Void
    let onRemove: (String) -> Void
    @State private var typed = ""
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            ForEach(servers, id: \.self) { server in
                HStack {
                    Text(server).frame(maxWidth: .infinity, alignment: .leading)
                    Button { onRemove(server) } label: { Image(.close).frame(width: 48, height: 48) }
                        .buttonStyle(.borderless)
                        .accessibilityLabel("Remove \(server)")
                }
                .frame(minHeight: 48)
            }
            HStack(alignment: .bottom) {
                LinkTextField(label: "Also send to", value: typed, placeholder: "127.0.0.1:14550", keyboard: .URL, onSubmit: { if !typed.isBlank { add() } }) { typed = $0 }
                Button(action: add) { Image(.add).frame(width: 44, height: 36) }
                    .buttonStyle(.borderless)
                    .disabled(typed.isBlank)
                    .accessibilityLabel("Add address")
            }
            Text("Optional. One address per device that should get telemetry too.").font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
        }
    }

    private func add() {
        onAdd(typed.trimmed)
        typed = ""
    }
}
