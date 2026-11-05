import SwiftUI

private func checkState(_ level: Int) -> SetupState { level == DeviceLevel.Ready ? .Done : .NeedsAttention }

// The Device tab's first page in Settings: the device's status and its pages, or the address to connect to. A
// page opened here is pushed onto the Settings NavigationStack (onOpen), where DevicePageScreen draws it.
struct DeviceHomeScreen: View {
    let onOpen: (String) -> Void
    @DeviceView("aircraft") private var reading

    var body: some View {
        Group {
            if reading.origin.isEmpty {
                ConnectDevice()
            } else {
                NativeForm {
                    if let status = deviceView(reading, parseDeviceStatus) {
                        DeviceHome(status: status, onOpen: onOpen)
                    } else {
                        FootNote(text: DEVICE_WAITING)
                    }
                    HostedDevice(host: reading.host)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .environment(\.LocalDeviceOrigin, reading.origin)
    }
}

// The device stack inside the Settings NavigationStack: the device hrefs above the Device tab, in order.
// onOpenFrom replaces the top page with returnTo and opens href above it, as the device's own pages expect.
func deviceRoutesOpenedFrom(_ hrefs: [String], _ href: String, _ returnTo: String) -> [String] { openedFrom(hrefs, href, returnTo) }

// One device page pushed onto the Settings NavigationStack; an href with no screen here shows the device's home.
struct DevicePageScreen: View {
    let href: String
    let onOpen: (String) -> Void
    let onOpenFrom: (_ href: String, _ returnTo: String) -> Void
    let onClose: () -> Void
    @DeviceView("aircraft") private var reading

    var body: some View {
        let opened = devicePath(href)
        Group {
            if let entry = DEVICE_SCREENS[opened.path] {
                NativeDevicePage(
                    entry: entry,
                    props: DeviceScreenProps(href: href, query: opened.query, onOpen: onOpen, onOpenFrom: onOpenFrom, onClose: onClose)
                )
                .id(href)
                .nativePage(entry.title)
            } else {
                DeviceHomeScreen(onOpen: onOpen).nativePage(DEVICE_PAGE)
            }
        }
        .environment(\.LocalDeviceOrigin, reading.origin)
    }
}

private struct DeviceHome: View {
    let status: DeviceStatus
    let onOpen: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Section {
            VStack(alignment: .leading, spacing: 4) {
                Text(status.headline).font(.headline).foregroundStyle(deviceLevelColor(status.level, theme))
                if !status.detail.isEmpty {
                    Text(status.detail).font(.subheadline).foregroundStyle(Color.secondary)
                }
                if let action = status.action {
                    Button(action.label) { onOpen(action.href) }.buttonStyle(.bordered).padding(.top, 8)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if let setup = status.setup {
                SetupRow(title: setup.label, state: .NeedsAttention, onClick: { onOpen(setup.href) }, icon: .build)
            }
        }
        Section {
            ForEach(status.checks, id: \.label) { check in
                SetupRow(title: check.label, status: check.value, state: checkState(check.level), onClick: check.href.isEmpty ? nil : { onOpen(check.href) })
            }
            SetupRow(
                title: status.events.ifEmpty("Flight log"),
                state: status.events.isEmpty ? .Neutral : .NeedsAttention,
                onClick: { onOpen("/flights") }
            )
        } header: {
            SectionHeader(text: "Before you fly")
        }
        if let link = status.link { UplinkSection(link: link) }
        if let health = status.health { HealthSection(health: health) }
        Section {
            ForEach(status.pages, id: \.href) { page in
                SetupRow(
                    title: page.title,
                    status: page.summary,
                    state: page.needs ? .NeedsAttention : .Neutral,
                    onClick: { onOpen(page.href) },
                    icon: devicePageIcon(page.icon)
                )
            }
        } header: {
            SectionHeader(text: status.name.ifEmpty("Device"))
        }
    }
}

private struct ConnectDevice: View {
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
        NativeForm {
            Section {
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
            } header: {
                SectionHeader(text: "Connect to an Aircast device")
            } footer: {
                Text("Type the address of a device on this network. Its status, cameras and settings open here.")
            }
            Section {
                Button("Connect", action: connect)
                    .buttonStyle(.borderedProminent)
                    .disabled(busy)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .listRowBackground(Color.clear)
            }
        }
    }
}

private struct HostedDevice: View {
    let host: String

    var body: some View {
        DeviceSection(title: "This app") {
            DeviceFact(label: "Connected to", value: host)
            Button("Disconnect") { DeviceLink.disconnect() }
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

// A device page's own layout (phase 2 moves these into NativeForm); the Settings NavigationStack draws its bar.
private struct NativeDevicePage: View {
    let entry: DeviceScreenEntry
    let props: DeviceScreenProps

    var body: some View {
        // Each page is its own NativeForm, which scrolls itself: a Form inside a ScrollView collapses to nothing.
        // The reader stays so pages can scroll to a row (?flight=, #section) through LocalDeviceScroll.
        ScrollViewReader { proxy in
            entry.screen(props)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
                .environment(\.LocalDeviceScroll, proxy)
        }
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
            .macTooltip("\(shown.name): \(shown.headline)")
        }
    }
}
