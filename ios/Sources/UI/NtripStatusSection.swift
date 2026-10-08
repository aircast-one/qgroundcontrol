import SwiftUI

let NTRIP_VIEW = "view.ntrip"
let NTRIP_CONNECT_SETTING = "settings.ntripSettings.ntripServerConnectEnabled"
private let NTRIP_POLL_MS = 1000
private let NTRIP_GREEN = Color(hex: 0x34C759)
private let NTRIP_ORANGE = Color(hex: 0xFF9F0A)

struct NtripStatus {
    let status: String
    let message: String
    let button: String
    let buttonEnabled: Bool
    let active: Bool
    let dataStale: Bool
    let mountpoint: String
    let messages: Int64
    let messageTypes: [(id: Int, count: Int64)]
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
        messageTypes: view["messageTypes"].array.compactMap { pair in
            pair.arrayOrNil.map { _ in (id: pair[0].int(0), count: pair[1].int64 ?? 0) }
        },
        bytesReceived: view["bytesReceived"].int64 ?? 0,
        dataRate: view["dataRateBytesPerSec"].double(0),
        dataWarning: view["dataWarning"].bool,
        bytesSent: view["bytesSent"].int64 ?? 0,
        sentKBps: view["sentKBps"].double(0),
        securityWarning: view["securityWarning"].string,
        ggaSource: view["ggaSource"].string,
        browser: ntripBrowser(view["browser"].object != nil ? view["browser"] : nil)
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
        mountpoints: (json?["mountpoints"].array ?? []).filter { $0.object != nil }.map {
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

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let status = read {
                SectionHeader(text: "Connection")
                details(status)
            }
        }
        .task {
            while !Task.isCancelled {
                read = await offMain { ntripStatus(Qgc.get(NTRIP_VIEW)) }
                try? await Task.sleep(for: .milliseconds(NTRIP_POLL_MS))
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
                Button(status.button) {
                    let enable = !status.active
                    Task {
                        _ = await offMain { Qgc.set(NTRIP_CONNECT_SETTING, enable) }
                        onWrite()
                    }
                }
                .buttonStyle(.borderedProminent)
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

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let read = browser {
                mountpoints(read)
            }
        }
        .task(id: refreshes) {
            browser = await offMain { ntripStatus(Qgc.get(NTRIP_VIEW))?.browser }
            if browser?.status == "inProgress" {
                try? await Task.sleep(for: .milliseconds(NTRIP_POLL_MS))
                if !Task.isCancelled { refreshes += 1 }
            }
        }
    }

    private func mountpoints(_ read: NtripBrowser) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Button("Browse") {
                Task {
                    _ = await offMain { Qgc.invoke(NTRIP_FETCH_MOUNTPOINTS) }
                    refreshes += 1
                }
            }
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
                    Button(row.selected ? "Selected" : "Select") {
                        let mountpoint = row.mountpoint
                        Task {
                            _ = await offMain { Qgc.invoke(NTRIP_SELECT_MOUNTPOINT, mountpoint) }
                            refreshes += 1
                            onWrite()
                        }
                    }
                    .buttonStyle(.borderless)
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
