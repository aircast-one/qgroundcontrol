import SwiftUI

let NTRIP_VIEW = "view.ntrip"
let NTRIP_CONNECT_SETTING = "settings.ntripSettings.ntripServerConnectEnabled"
private let NTRIP_POLL_MS = 1000
private let NTRIP_GREEN = Color(hex: 0x34C759)
private let NTRIP_ORANGE = Color(hex: 0xFF9F0A)

struct NtripMessageType: Equatable {
    let id: Int
    let count: Int64
}

struct NtripStatus: Equatable {
    let status: String
    let message: String
    let button: String
    let buttonEnabled: Bool
    let active: Bool
    let dataStale: Bool
    let mountpoint: String
    let messages: Int64
    let messageTypes: [NtripMessageType]
    let bytesReceived: Int64
    let dataRate: Double
    let dataWarning: Bool
    let bytesSent: Int64
    let sentKBps: Double
    let securityWarning: String
    let ggaSource: String
    let browser: NtripBrowser
}

func ntripStatus(_ view: JSON?) -> NtripStatus? {
    guard let view, !view["status"].string.isBlank else { return nil }
    return NtripStatus(
        status: view["status"].string,
        message: view["statusMessage"].string,
        button: view["button"].string,
        buttonEnabled: view["buttonEnabled"].bool,
        active: view["active"].bool,
        dataStale: view["dataStale"].bool,
        mountpoint: view["mountpoint"].string,
        messages: view["messages"].int64 ?? 0,
        messageTypes: view["messageTypes"].array.filter { $0.arrayOrNil != nil }.map { pair in
            NtripMessageType(id: pair[0].int(0), count: pair[1].int64 ?? 0)
        },
        bytesReceived: view["bytesReceived"].int64 ?? 0,
        dataRate: view["dataRateBytesPerSec"].double(0),
        dataWarning: view["dataWarning"].bool,
        bytesSent: view["bytesSent"].int64 ?? 0,
        sentKBps: view["sentKBps"].double(0),
        securityWarning: view["securityWarning"].string,
        ggaSource: view["ggaSource"].string,
        browser: ntripBrowser(view["browser"].objectOrNil)
    )
}

struct NtripMountpointRow: Equatable {
    let mountpoint: String
    let detail: String
    let selected: Bool
}

struct NtripBrowser: Equatable {
    let status: String
    let error: String
    let canBrowse: Bool
    let mountpoints: [NtripMountpointRow]
}

func ntripBrowser(_ json: JSON?) -> NtripBrowser {
    NtripBrowser(
        status: json?["status"].string ?? "",
        error: json?["error"].string ?? "",
        canBrowse: json?["canBrowse"].bool == true,
        mountpoints: (json?["mountpoints"].objects ?? []).map {
            NtripMountpointRow(mountpoint: $0["mountpoint"].string, detail: $0["detail"].string, selected: $0["selected"].bool)
        }
    )
}

func dataSize(_ bytes: Int64) -> String {
    switch bytes {
    case ..<1024: "\(bytes) B"
    case ..<1_048_576: String(format: "%.1f KB", Double(bytes) / 1024.0)
    default: String(format: "%.1f MB", Double(bytes) / 1_048_576.0)
    }
}

func dataRate(_ bytesPerSec: Double) -> String {
    bytesPerSec < 1024 ? String(format: "%.0f B/s", bytesPerSec) : String(format: "%.1f KB/s", bytesPerSec / 1024)
}

struct NtripStatusSection: View {
    let onWrite: () -> Void
    @State private var read: NtripStatus?
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            native
        } else {
            handDrawn
        }
    }

    private var handDrawn: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let status = read {
                SectionHeader(text: "Connection")
                details(status)
            }
        }
        .task { await poll() }
    }

    @MainActor private func poll() async {
        while !Task.isCancelled {
            let next = await offMain { ntripStatus(Qgc.get(NTRIP_VIEW)) }
            if next != read { read = next }
            try? await Task.sleep(for: .milliseconds(NTRIP_POLL_MS))
        }
    }

    @MainActor private func toggleConnection(_ status: NtripStatus) {
        let enable = !status.active
        Task {
            _ = await offMain { Qgc.set(NTRIP_CONNECT_SETTING, enable) }
            onWrite()
        }
    }

    // Inside a NativeForm: a Connection section — the state with Connect or Disconnect, then a value row per
    // reading while connected. The state row carries the poll, so it is always there.
    @ViewBuilder private var native: some View {
        Section {
            HStack(spacing: Space.s3) {
                Circle().fill(read.map { dot($0) } ?? theme.colors.outline).frame(width: 10, height: 10).accessibilityHidden(true)
                Text(read.map { $0.message.ifBlank("Disconnected") } ?? "Reading…").frame(maxWidth: .infinity, alignment: .leading)
                if let status = read {
                    Button(status.button) { toggleConnection(status) }
                        .buttonStyle(.bordered)
                        .disabled(!status.buttonEnabled)
                }
            }
            .task { await poll() }
            if let status = read, status.status != "disconnected" {
                nativeDetails(status)
            }
        } header: {
            SectionHeader(text: "Connection")
        }
    }

    @ViewBuilder private func nativeDetails(_ status: NtripStatus) -> some View {
        let connected = status.status == "connected"
        if status.dataStale && connected {
            Text("Connected but no data received recently").foregroundStyle(NTRIP_ORANGE)
        }
        if connected && !status.mountpoint.isBlank { LabeledContent("Mountpoint", value: status.mountpoint) }
        if connected && status.messages > 0 { LabeledContent("Messages", value: String(status.messages)) }
        if connected && !status.messageTypes.isEmpty {
            VStack(alignment: .leading, spacing: Space.s2) {
                Text("Message types")
                messageTypes(status)
            }
            .padding(.vertical, Space.s1)
        }
        if connected && status.bytesReceived > 0 {
            LabeledContent("Data Received", value: "\(dataSize(status.bytesReceived)) (\(dataRate(status.dataRate)))")
        }
        if connected && status.dataWarning {
            Text("Warning: Data usage: \(dataSize(status.bytesReceived)) — consider connection costs")
                .font(.footnote)
                .foregroundStyle(NTRIP_ORANGE)
        }
        if connected && status.bytesSent > 0 {
            LabeledContent("To Vehicle", value: "\(dataSize(status.bytesSent)) (\(String(format: "%.1f", status.sentKBps)) KB/s)")
        }
        if connected && !status.ggaSource.isBlank { LabeledContent("GGA Source", value: status.ggaSource) }
        if !status.securityWarning.isBlank {
            Text(status.securityWarning).font(.footnote).foregroundStyle(NTRIP_ORANGE)
        }
        if status.status == "error" {
            Text(status.message).font(.footnote).foregroundStyle(theme.colors.error)
        }
    }

    private func messageTypes(_ status: NtripStatus) -> some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 88), spacing: 6, alignment: .leading)], alignment: .leading, spacing: 6) {
            ForEach(Array(status.messageTypes.enumerated()), id: \.offset) { _, type in
                Text("\(type.id == 0 ? "unknown" : String(type.id)) × \(type.count)")
                    .font(.bodySmall)
                    .padding(.horizontal, Space.s2)
                    .padding(.vertical, 2)
                    .background(theme.colors.surfaceVariant, in: Capsule())
            }
        }
    }

    private func dot(_ status: NtripStatus) -> Color {
        switch status.status {
        case "connected": NTRIP_GREEN
        case "connecting", "reconnecting": NTRIP_ORANGE
        case "error": theme.colors.error
        default: theme.colors.outline
        }
    }

    private func details(_ status: NtripStatus) -> some View {
        let connected = status.status == "connected"
        return VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 10) {
                Circle().fill(dot(status)).frame(width: 10, height: 10)
                Text(status.message.ifBlank("Disconnected")).frame(maxWidth: .infinity, alignment: .leading)
                Button(status.button) { toggleConnection(status) }
                .buttonStyle(.filled)
                .disabled(!status.buttonEnabled)
            }
            if status.status != "disconnected" {
                if status.dataStale && connected {
                    Text("Connected but no data received recently").foregroundStyle(NTRIP_ORANGE)
                }
                if connected && !status.mountpoint.isBlank { StatusLine(label: "Mountpoint", value: status.mountpoint) }
                if connected && status.messages > 0 { StatusLine(label: "Messages", value: String(status.messages)) }
                if connected && !status.messageTypes.isEmpty {
                    Text("Message types").font(.labelLarge)
                    messageTypes(status)
                }
                if connected && status.bytesReceived > 0 {
                    StatusLine(label: "Data Received", value: "\(dataSize(status.bytesReceived)) (\(dataRate(status.dataRate)))")
                }
                if connected && status.dataWarning {
                    Text("Warning: Data usage: \(dataSize(status.bytesReceived)) — consider connection costs")
                        .font(.bodySmall)
                        .foregroundStyle(NTRIP_ORANGE)
                }
                if connected && status.bytesSent > 0 {
                    StatusLine(label: "To Vehicle", value: "\(dataSize(status.bytesSent)) (\(String(format: "%.1f", status.sentKBps)) KB/s)")
                }
                if connected && !status.ggaSource.isBlank { StatusLine(label: "GGA Source", value: status.ggaSource) }
                if !status.securityWarning.isBlank {
                    Text(status.securityWarning).font(.bodySmall).foregroundStyle(NTRIP_ORANGE)
                }
                if status.status == "error" {
                    Text(status.message).font(.bodySmall).foregroundStyle(theme.colors.error)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, Space.s5)
        .padding(.vertical, Space.s2)
    }
}

private struct StatusLine: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
            Text(value).font(.bodyMedium)
        }
    }
}

let NTRIP_FETCH_MOUNTPOINTS = "ntrip.fetchMountpoints"
let NTRIP_SELECT_MOUNTPOINT = "ntrip.selectMountpoint"

struct NtripMountpointBrowser: View {
    let onWrite: () -> Void
    @State private var browser: NtripBrowser?
    @State private var refreshes = 0
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            native
        } else {
            handDrawn
        }
    }

    private var handDrawn: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let read = browser {
                mountpoints(read)
            }
        }
        .task(id: refreshes) { await refresh() }
    }

    @MainActor private func refresh() async {
        browser = await offMain { ntripStatus(Qgc.get(NTRIP_VIEW))?.browser }
        if browser?.status == "inProgress" {
            try? await Task.sleep(for: .milliseconds(NTRIP_POLL_MS))
            if !Task.isCancelled { refreshes += 1 }
        }
    }

    @MainActor private func browse() {
        Task {
            _ = await offMain { Qgc.invoke(NTRIP_FETCH_MOUNTPOINTS) }
            refreshes += 1
        }
    }

    @MainActor private func select(_ mountpoint: String) {
        Task {
            _ = await offMain { Qgc.invoke(NTRIP_SELECT_MOUNTPOINT, mountpoint) }
            refreshes += 1
            onWrite()
        }
    }

    // Inside a NativeForm: rows of the mountpoint section — Browse (which carries the read), the fetch state, and a
    // row per mountpoint, the selected one checked.
    @ViewBuilder private var native: some View {
        Button("Browse mountpoints") { browse() }
            .disabled(browser?.canBrowse != true)
            .task(id: refreshes) { await refresh() }
        if let read = browser {
            if read.status == "inProgress" { Text("Fetching mountpoints…").foregroundStyle(NTRIP_ORANGE) }
            if read.status == "error" { Text(read.error).foregroundStyle(theme.colors.error) }
            ForEach(read.mountpoints, id: \.mountpoint) { row in
                Button { select(row.mountpoint) } label: {
                    HStack(spacing: Space.s3) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(row.mountpoint).foregroundStyle(Color.primary)
                            if !row.detail.isBlank {
                                Text(row.detail).font(.subheadline).foregroundStyle(Color.secondary)
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        if row.selected {
                            Image(systemName: "checkmark").fontWeight(.semibold).foregroundStyle(theme.colors.primary)
                        }
                    }
                    .contentShape(Rectangle())
                }
                .listRowButton()
                .disabled(row.selected)
                .accessibilityAddTraits(row.selected ? .isSelected : [])
            }
        }
    }

    private func mountpoints(_ read: NtripBrowser) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Button("Browse") { browse() }
            .buttonStyle(.bordered)
            .disabled(!read.canBrowse)
            if read.status == "inProgress" { Text("Fetching mountpoints…").foregroundStyle(NTRIP_ORANGE) }
            if read.status == "error" { Text(read.error).foregroundStyle(theme.colors.error) }
            ForEach(read.mountpoints, id: \.mountpoint) { row in
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        HStack(spacing: 6) {
                            Text(row.mountpoint)
                                .font(.bodyLarge)
                                .foregroundStyle(row.selected ? theme.colors.primary : theme.colors.onSurface)
                            if row.selected { Text("(selected)").font(.bodySmall).foregroundStyle(theme.colors.primary) }
                        }
                        if !row.detail.isBlank {
                            Text(row.detail).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    Button(row.selected ? "Selected" : "Select") { select(row.mountpoint) }
                    .buttonStyle(.text)
                    .disabled(row.selected)
                }
                .padding(.vertical, Space.s1)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, Space.s5)
        .padding(.vertical, Space.s2)
    }
}
