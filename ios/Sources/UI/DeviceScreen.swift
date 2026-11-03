import SwiftUI

private func checkState(_ level: Int) -> SetupState { level == DeviceLevel.Ready ? .Done : .NeedsAttention }

struct DeviceTab: View {
    let open: String?
    let onOpen: (String) -> Void
    let onClose: () -> Void

    var body: some View {
        DeviceStack(open: open, onOpen: onOpen, onClose: onClose).id(open)
    }
}

private struct DeviceStack: View {
    let onOpen: (String) -> Void
    let onClose: () -> Void
    @DeviceView("aircraft") private var reading
    @State private var stack: [String]

    init(open: String?, onOpen: @escaping (String) -> Void, onClose: @escaping () -> Void) {
        self.onOpen = onOpen
        self.onClose = onClose
        _stack = State(initialValue: open.map { [$0] } ?? [])
    }

    private func back() {
        if stack.count > 1 { stack.removeLast() } else { onClose() }
    }

    var body: some View {
        let current = stack.last
        let opened = current.map(devicePath)
        let entry = opened.flatMap { DEVICE_SCREENS[$0.path] }
        Group {
            if let current, let opened, let entry {
                NativeDevicePage(
                    entry: entry,
                    props: DeviceScreenProps(
                        href: current,
                        query: opened.query,
                        onOpen: { stack.append($0) },
                        onOpenFrom: { href, returnTo in stack = openedFrom(stack, href, returnTo) },
                        onClose: back
                    ),
                    onBack: back
                )
                .id(current)
            } else if reading.origin.isEmpty {
                ConnectDevice()
            } else {
                ScrollView {
                    VStack(alignment: .leading, spacing: 0) {
                        if let status = deviceView(reading, parseDeviceStatus) {
                            DeviceHome(status: status, onOpen: onOpen)
                        } else {
                            FootNote(text: DEVICE_WAITING)
                        }
                        HostedDevice(host: reading.host)
                    }
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .environment(\.LocalDeviceOrigin, reading.origin)
    }
}

private struct DeviceHome: View {
    let status: DeviceStatus
    let onOpen: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(status.headline).font(.titleMedium).foregroundStyle(deviceLevelColor(status.level, theme))
            if !status.detail.isEmpty {
                Text(status.detail).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            if let action = status.action {
                Button(action.label) { onOpen(action.href) }.buttonStyle(.bordered).padding(.top, 8)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(EdgeInsets(top: 16, leading: 16, bottom: 8, trailing: 16))
        if let setup = status.setup {
            SetupRow(title: setup.label, state: .NeedsAttention, onClick: { onOpen(setup.href) }, icon: .build)
        }
        SectionHeader(text: "Before you fly")
        ForEach(status.checks, id: \.label) { check in
            SetupRow(title: check.label, status: check.value, state: checkState(check.level), onClick: check.href.isEmpty ? nil : { onOpen(check.href) })
        }
        SetupRow(
            title: status.events.ifEmpty("Flight log"),
            state: status.events.isEmpty ? .Neutral : .NeedsAttention,
            onClick: { onOpen("/flights") }
        )
        if let link = status.link { UplinkSection(link: link) }
        if let health = status.health { HealthSection(health: health) }
        SectionHeader(text: status.name.ifEmpty("Device"))
        ForEach(status.pages, id: \.href) { page in
            SetupRow(
                title: page.title,
                status: page.summary,
                state: page.needs ? .NeedsAttention : .Neutral,
                onClick: { onOpen(page.href) },
                icon: devicePageIcon(page.icon)
            )
        }
    }
}

private struct ConnectDevice: View {
    @Environment(\.theme) private var theme
    @State private var address = ""
    @State private var busy = false
    @State private var refusal = ""

    private func connect() {
        busy = true
        let typed = address
        Task { @MainActor in
            refusal = await DeviceLink.connect(typed) ?? ""
            busy = false
        }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                Text("Connect to an Aircast device").font(.titleMedium)
                Text("Type the address of a device on this network. Its status, cameras and settings open here.")
                    .font(.bodyMedium)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                DeviceTextField(
                    label: "Device address",
                    value: address,
                    onValueChange: { address = $0 },
                    placeholder: "192.168.1.100 or falcon.local",
                    isError: !refusal.isEmpty,
                    supporting: refusal,
                    keyboard: .uri,
                    onSubmit: connect
                )
                Button("Connect", action: connect).buttonStyle(.bordered).disabled(busy)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(16)
        }
    }
}

private struct HostedDevice: View {
    let host: String

    var body: some View {
        DeviceSection(title: "This app") {
            DeviceFact(label: "Connected to", value: host)
            Button("Disconnect") { DeviceLink.disconnect() }.buttonStyle(.text).padding(.horizontal, 8)
        }
    }
}

private struct UplinkSection: View {
    let link: DeviceUplink

    var body: some View {
        DeviceSection(title: "Link") {
            DeviceFact(label: "Carrying video", value: link.carrier)
            DeviceText(text: [link.backup, link.network].filter { !$0.isEmpty }.joined(separator: " · "))
            if !link.rate.isEmpty { DeviceFact(label: "Throughput", value: link.rate) }
            if !link.standby.isEmpty { DeviceFact(label: "Cellular · standby", value: link.standby) }
            if !link.trouble.isEmpty { DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Warn, text: link.trouble)) }
            if !link.data.isEmpty { DeviceFact(label: "Data", value: link.data) }
            ForEach(link.charts, id: \.label) { DeviceChart(chart: $0) }
        }
    }
}

private struct HealthSection: View {
    let health: DeviceHealth

    var body: some View {
        DeviceSection(title: "Device health") {
            if !health.memory.isEmpty { DeviceFact(label: "Memory in use", value: health.memory) }
            if let warning = health.warning { DeviceNotice(notice: warning) }
            ForEach(health.charts, id: \.label) { DeviceChart(chart: $0) }
        }
    }
}

private struct NativeDevicePage: View {
    let entry: DeviceScreenEntry
    let props: DeviceScreenProps
    let onBack: () -> Void

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                VStack(alignment: .leading, spacing: 0) { entry.screen(props) }
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .environment(\.LocalDeviceScroll, proxy)
        }
        .background(OverridePageHeading(title: entry.title, onBack: onBack))
    }
}

func chipText(_ status: DeviceStatus) -> String { status.name.isEmpty ? status.headline : "\(status.name) · \(status.headline)" }

struct DeviceStatusChip: View {
    @Environment(AppNavigationState.self) private var navigation
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd
    @FlyIsPortrait private var portrait
    @DeviceView("aircraft") private var reading

    private func open() {
        navigation.settingsPage = DEVICE_PAGE
        navigation.settingsOpen = true
    }

    var body: some View {
        if let shown = deviceView(reading, parseDeviceStatus), deviceNeedsAttention(shown) {
            let backdrop = osdBackdrop(Color.black.opacity(0.45), flyOsd)
            Button(action: open) {
                if portrait {
                    Image(.developerBoard)
                        .font(.system(size: 20))
                        .frame(width: 40, height: 40)
                        .overlay(alignment: .topTrailing) {
                            Circle()
                                .fill(deviceLevelColor(shown.level, theme))
                                .frame(width: 10, height: 10)
                                .overlay(Circle().stroke(Color.black.opacity(0.4), lineWidth: 2))
                                .padding(6)
                        }
                        .background(backdrop, in: Circle())
                } else {
                    HStack(spacing: 8) {
                        Circle().fill(deviceLevelColor(shown.level, theme)).frame(width: 8, height: 8)
                        Text(chipText(shown)).font(.labelLarge).lineLimit(1).truncationMode(.tail)
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .frame(maxWidth: 320)
                    .background(backdrop, in: Capsule())
                }
            }
            .buttonStyle(.plain)
            .foregroundStyle(theme.aircast.outdoorForeground)
            .accessibilityLabel("\(shown.name): \(shown.headline)")
        }
    }
}
