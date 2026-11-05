import SwiftUI

private struct WifiControls: View {
    let wifi: WifiCard

    var body: some View {
        if wifi.saved != nil || wifi.hotspot != nil {
            DeviceButtonRow {
                if let saved = wifi.saved {
                    ActionButton(
                        label: saved.label,
                        request: DeviceRequest(method: .POST, path: "/api/network/wifi/saved"),
                        success: "Joined. The device switches Wi-Fi by itself again.",
                        confirm: ActionConfirm(title: "\(saved.label)?", text: saved.confirm, confirm: "Join")
                    )
                }
                if let hotspot = wifi.hotspot {
                    ActionButton(
                        label: hotspot.label,
                        request: DeviceRequest(method: .POST, path: "/api/network/wifi/hotspot"),
                        success: "Hotspot on until the device restarts.",
                        confirm: ActionConfirm(title: "Keep the hotspot on?", text: hotspot.confirm, confirm: "Use hotspot"),
                        tonal: false
                    )
                }
            }
        }
    }
}

struct DeviceNetworkScreen: View {
    let props: DeviceScreenProps
    @DeviceView("network") private var reading

    var body: some View {
        NativeForm {
            if let view = deviceView(reading, parseNetwork) {
                PrimaryUplinkSection(uplink: view.uplink, onOpen: props.onOpen)
                if let modem = view.modem { ModemSection(modem: modem) }
                StarlinkSection(starlink: view.starlink)
                DeviceSection(title: "Wi-Fi", badge: view.wifi.status) {
                    NetSummary(text: view.wifi.summary)
                    if let joined = view.wifi.joined { DeviceNotice(notice: joined) }
                    if let held = view.wifi.held { DeviceNotice(notice: held) }
                    WifiControls(wifi: view.wifi)
                    WifiJoin(reopen: view.wifi.reopen)
                }
                InterfacesSection(interfaces: view.interfaces)
                FootNote(text: "The device routes through the best uplink that reaches the internet and moves off one that stops — this is where you say what it may use.")
                DataSection(data: view.data)
                FootNote(text: "Mobile data is metered; the budget here is what keeps a flight from spending a month of it — set it before a SIM goes in.")
                Group {
                    TailscaleSection(tailscale: view.tailscale)
                    RelaySection(relay: view.relay)
                    FootNote(text: "Reaching this device from somewhere else — it is not needed to fly.")
                }
            } else {
                FootNote(text: DEVICE_WAITING)
            }
        }
    }
}

private struct NetSummary: View {
    let text: String
    @Environment(\.theme) private var theme

    var body: some View {
        if !text.isEmpty {
            Text(text)
                .font(.bodyMedium)
                .foregroundStyle(theme.colors.onSurfaceVariant)
                .frame(maxWidth: .infinity, alignment: .leading)
                .rowPadding(EdgeInsets(top: 0, leading: DEVICE_LIST_PADDING, bottom: 4, trailing: DEVICE_LIST_PADDING))
        }
    }
}

private struct NetLeveled: View {
    let badge: DeviceBadge
    @Environment(\.theme) private var theme

    var body: some View {
        Text(badge.text).font(.bodyMedium).foregroundStyle(deviceLevelColor(badge.level, theme))
    }
}

private func neutralOr(_ level: Int, _ neutral: Color, _ theme: Theme) -> Color {
    level == DeviceLevel.Neutral ? neutral : deviceLevelColor(level, theme)
}

private struct NetMono: View {
    let text: String

    var body: some View {
        Text(text)
            .font(deviceMono(.bodySmall))
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
            .rowPadding(EdgeInsets(top: 0, leading: DEVICE_LIST_PADDING, bottom: 8, trailing: DEVICE_LIST_PADDING))
    }
}

private struct ProblemNotice: View {
    let problem: DeviceProblem

    var body: some View {
        DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Blocked, text: problem.text))
        if !problem.detail.isEmpty {
            DeviceGroup("Technical detail") { NetMono(text: problem.detail) }
        }
    }
}

private struct PrimaryUplinkSection: View {
    let uplink: UplinkCard
    let onOpen: (String) -> Void
    @Environment(\.theme) private var theme

    private var reach: String { [uplink.addr, uplink.meta].filter { !$0.isEmpty }.joined(separator: " · ") }

    var body: some View {
        DeviceSection(title: "Primary uplink", badge: uplink.status) {
            HStack(alignment: .top, spacing: 12) {
                Image(uplinkIcon(uplink.icon))
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .frame(width: 24, height: 24)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 4) {
                    Text(uplink.title).font(.titleMedium).accessibilityAddTraits(.isHeader)
                    NetLeveled(badge: uplink.verdict)
                    if !uplink.detail.isEmpty {
                        Text(uplink.detail).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                    if !reach.isEmpty {
                        Text(reach).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                if !uplink.rate.isEmpty {
                    VStack(alignment: .trailing, spacing: 2) {
                        Text("Sending now").font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
                        Text(uplink.rate).font(deviceMono(.titleMedium))
                    }
                }
            }
            .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
            if !uplink.warning.isEmpty {
                DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Warn, text: uplink.warning))
                SetupRow(title: "Check the camera", onClick: { onOpen("/streaming") })
            }
            ForEach(uplink.failover, id: \.label) { row in
                LabeledContent(row.label) { NetLeveled(badge: row.status) }
            }
            DeviceText(text: uplink.switchText)
            DeviceText(text: uplink.backup)
        }
    }
}

private struct ModemSection: View {
    let modem: ModemCard
    @Environment(\.theme) private var theme

    var body: some View {
        DeviceSection(title: "Cellular modem", badge: modem.status) {
            ForEach(modem.notices, id: \.text) { DeviceNotice(notice: $0) }
            VStack(alignment: .leading, spacing: 0) {
                Text(modem.operatorName).font(.bodyLarge)
                Text(modem.detail).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
            if let signal = modem.signal { DeviceChart(chart: signal) }
            ForEach(modem.facts, id: \.label) { DeviceFact(label: $0.label, value: $0.value) }
            DeviceGroup("Text messages", summary: modem.messages.summary) {
                if modem.messages.items.isEmpty {
                    DeviceText(text: "Messages the operator sends to this SIM — package expiry, top-ups, portal codes — show up here.")
                } else {
                    ForEach(Array(modem.messages.items.enumerated()), id: \.offset) { _, message in
                        SmsRow(message: message)
                    }
                }
            }
            if let sim = modem.sim {
                DeviceGroup("SIM & APN", summary: sim.summary) {
                    DeviceForm(
                        fields: [
                            DeviceField(name: "apn", label: "APN", value: sim.apn, placeholder: sim.placeholder),
                            DeviceField(name: "pin", label: "SIM PIN", value: "", hint: "Leave blank to keep the current PIN.", keyboard: .password),
                            DeviceField(name: "metered", label: "Data usage", value: sim.metered, options: sim.options),
                        ],
                        method: .PUT,
                        path: "/api/network/cellular",
                        submit: "Save",
                        success: "Cellular settings applied.",
                        confirm: ActionConfirm(
                            title: "Re-dial the modem?",
                            text: "Saving re-dials the modem — the cellular link drops briefly. If you're connected over cellular you may lose access until it reconnects. Continue?",
                            confirm: "Save"
                        ),
                        note: "Re-dials the modem on save."
                    )
                    .id([sim.apn, sim.metered])
                }
            }
        }
    }
}

private struct SmsRow: View {
    let message: SmsMessage
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 12) {
                Text(message.from).font(.bodyLarge).frame(maxWidth: .infinity, alignment: .leading)
                Text(message.time).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            Text(message.text).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant).textSelection(.enabled)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
    }
}

private struct StarlinkSection: View {
    let starlink: StarlinkCard

    var body: some View {
        DeviceSection(title: "Starlink", badge: starlink.status) {
            NetSummary(text: starlink.summary)
            if let demo = starlink.demo, demo.first { DemoDish(demo: demo) }
            ForEach(starlink.notices, id: \.text) { DeviceNotice(notice: $0) }
            if !starlink.pollError.isEmpty { NetMono(text: starlink.pollError) }
            ForEach(starlink.facts, id: \.label) { DeviceFact(label: $0.label, value: $0.value) }
            if !starlink.firmware.isEmpty { DeviceFact(label: "Firmware", value: starlink.firmware) }
            DeviceGroup("Dish address", initiallyOpen: starlink.address.open) {
                DeviceText(
                    text: "The network address of the Starlink dish. Leave the default unless the dish is reached at a different address on this device's network. Port 9200 is assumed if you leave it off."
                )
                if starlink.address.defaultFirst { UseDefaultAddress(fallback: starlink.address.fallback) }
                DeviceForm(
                    fields: [DeviceField(name: "addr", label: "Dish address", value: starlink.address.value, placeholder: "192.168.100.1:9200", keyboard: .uri)],
                    method: .PUT,
                    path: "/api/starlink/config",
                    submit: "Save",
                    success: "Dish address saved."
                )
                .id(starlink.address.value)
                if !starlink.address.defaultFirst { UseDefaultAddress(fallback: starlink.address.fallback) }
            }
            if let demo = starlink.demo, !demo.first { DemoDish(demo: demo) }
        }
    }
}

private struct UseDefaultAddress: View {
    let fallback: String

    var body: some View {
        if !fallback.isEmpty {
            DeviceButtonRow {
                ActionButton(
                    label: "Use default (\(fallback))",
                    request: DeviceRequest(method: .PUT, path: "/api/starlink/config", body: deviceBody(["addr": fallback])),
                    success: "Dish address back to the default."
                )
            }
        }
    }
}

private func demoCondition(_ condition: String) -> DeviceRequest {
    DeviceRequest(method: .POST, path: "/api/starlink/demo", body: deviceBody(["condition": condition]))
}

private struct DemoDish: View {
    let demo: StarlinkDemo
    @Environment(\.LocalChangeNotice) private var notice
    @State private var action = DeviceActionState()

    private func pick(_ condition: String) {
        let action = action, notice = notice
        Task { @MainActor in _ = await action.run(demoCondition(condition), success: "Demo dish condition changed.", notice: notice) }
    }

    var body: some View {
        DeviceSwitchRow(
            title: "Demo dish",
            checked: demo.on,
            on: demoCondition("healthy"),
            off: demoCondition("off"),
            detail: demo.note,
            success: "Demo dish on — simulated, not a real dish.",
            offSuccess: "Demo dish off."
        )
        if demo.on {
            DeviceSelect(label: "Condition", value: demo.condition, options: demo.conditions, onPick: pick, enabled: !action.busy)
                .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
            DeviceRefusalText(refusal: action.refusal, field: action.refusal?.field ?? "")
        }
    }
}

private func ipv4Fields(_ methods: [DeviceOption], _ ipv4: IPv4Settings) -> [DeviceField] {
    [
        DeviceField(name: "method", label: "IPv4 method", value: ipv4.method.ifEmpty(methods.first?.value ?? "auto"), options: methods),
        DeviceField(name: "metric", label: "Route metric (priority)", value: ipv4.metric, placeholder: ipv4.metricPlaceholder, keyboard: .number),
        DeviceField(
            name: "address",
            label: "Address (CIDR)",
            value: ipv4.address,
            placeholder: "192.168.8.2/24",
            keyboard: .uri,
            shownWhen: { manualAddressing($0["method"] ?? "") }
        ),
        DeviceField(name: "gateway", label: "Gateway", value: ipv4.gateway, placeholder: "optional", keyboard: .uri, shownWhen: { manualAddressing($0["method"] ?? "") }),
        DeviceField(name: "dns", label: "DNS", value: ipv4.dns, placeholder: "8.8.8.8, 1.1.1.1", keyboard: .uri),
    ]
}

private func newConnectionField(_ name: String, _ label: String, placeholder: String = "", hint: String = "", keyboard: DeviceKeyboard = .text) -> DeviceField {
    DeviceField(name: name, label: label, value: "", placeholder: placeholder, hint: hint, keyboard: keyboard, shownWhen: { connectionFieldShown($0["type"] ?? "", name) })
}

private struct RenameForm: View {
    let uuid: String
    let name: String

    var body: some View {
        DeviceForm(
            fields: [DeviceField(name: "name", label: "Connection name", value: name, placeholder: "e.g. Wired connection 2")],
            method: .PUT,
            path: "/api/network/connections/\(uuid)/name",
            submit: "Rename",
            success: "Connection renamed."
        )
        .id(name)
    }
}

private extension NetLink {
    var rowKey: String { editor.flatMap { $0.uuid.isEmpty ? nil : $0.uuid } ?? name }
}

private struct InterfacesSection: View {
    let interfaces: InterfacesCard
    @Environment(\.theme) private var theme

    private var listed: Bool { interfaces.links.count + interfaces.saved.count + interfaces.other.count > 0 }

    private var addFields: [DeviceField] {
        [
            DeviceField(name: "type", label: "Type", value: interfaces.add.types.first?.value ?? "ethernet", options: interfaces.add.types),
            DeviceField(name: "name", label: "Name", value: "", placeholder: "camera-net"),
            DeviceField(name: "interface", label: "Interface", value: "", options: [DeviceOption(value: "", label: "Any interface")] + interfaces.add.ports),
            DeviceField(name: "autoconnect", label: "Autoconnect", value: interfaces.add.autoconnect.first?.value ?? "false", options: interfaces.add.autoconnect),
            newConnectionField("ssid", "SSID", placeholder: "network name"),
            newConnectionField("psk", "Passphrase", hint: "Leave blank for an open network.", keyboard: .password),
            newConnectionField("apn", "APN", placeholder: "internet"),
            newConnectionField("pin", "SIM PIN", placeholder: "optional", keyboard: .password),
        ] + ipv4Fields(interfaces.methods, IPv4Settings(method: "", metric: "", metricPlaceholder: "automatic", address: "", gateway: "", dns: ""))
    }

    var body: some View {
        DeviceSection(title: "Interfaces & priority", badge: interfaces.status) {
            NetSummary(text: interfaces.summary)
            if let problem = interfaces.problem { ProblemNotice(problem: problem) }
            if interfaces.problem == nil && !listed { DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Neutral, text: "No connections.")) }
            ForEach(interfaces.links, id: \.rowKey) { NetLinkRow(link: $0, methods: interfaces.methods) }
            ForEach(interfaces.saved, id: \.uuid) { SavedRow(profile: $0, methods: interfaces.methods) }
            if interfaces.problem == nil && interfaces.links.count + interfaces.saved.count > 0 {
                DeviceText(
                    text: "The primary uplink carries internet traffic; a backup takes over if it drops. Open a connection to edit it — changes to a running connection apply immediately; inactive ones are saved profiles that aren't running."
                )
            }
            if !interfaces.other.isEmpty {
                Text("Other interfaces")
                    .font(.labelMedium)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .rowPadding(EdgeInsets(top: 12, leading: DEVICE_LIST_PADDING, bottom: 0, trailing: DEVICE_LIST_PADDING))
                ForEach(interfaces.other, id: \.label) { usage in
                    VStack(alignment: .leading, spacing: 0) {
                        Text(usage.label).font(.bodyLarge)
                        Text("\(usage.month) · all-time \(usage.total)").font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
                }
            }
            DeviceGroup("Add a connection", summary: "Create a new NetworkManager profile — ethernet, Wi-Fi or cellular.") {
                DeviceForm(
                    fields: addFields,
                    method: .POST,
                    path: "/api/network/connections",
                    submit: "Create",
                    success: "Connection created.",
                    note: "Created inactive unless autoconnect is on — it won't disturb the current uplink."
                )
            }
        }
    }
}

private func linkSummary(_ link: NetLink) -> String {
    [link.kind, link.role?.text ?? (link.local ? "local" : ""), link.place, link.usage].filter { !$0.isEmpty }.joined(separator: " · ")
}

private struct NetLinkRow: View {
    let link: NetLink
    let methods: [DeviceOption]
    @Environment(\.theme) private var theme

    var body: some View {
        if let editor = link.editor {
            DeviceGroup(link.name, summary: linkSummary(link)) {
                DeviceForm(
                    fields: ipv4Fields(methods, editor.ipv4),
                    method: .PUT,
                    path: "/api/network/interfaces/\(editor.iface)",
                    submit: "Save",
                    success: "\(editor.iface) updated.",
                    confirm: editor.confirm.isEmpty ? nil : ActionConfirm(title: "Reconfigure \(editor.iface)?", text: editor.confirm, confirm: "Continue"),
                    note: "Applies immediately."
                )
                .id(editor.ipv4)
                if !editor.uuid.isEmpty {
                    RenameForm(uuid: editor.uuid, name: link.name)
                    DeviceText(text: "Bring this connection down, or remove the saved profile.")
                    DeviceButtonRow {
                        ActionButton(
                            label: "Deactivate",
                            request: DeviceRequest(method: .POST, path: "/api/network/connections/\(editor.uuid)/deactivate"),
                            success: "\(link.name) deactivated.",
                            confirm: ActionConfirm(title: "Deactivate \(link.name)?", text: editor.deactivateConfirm, confirm: "Deactivate")
                        )
                        DeviceDestructiveButton(
                            label: "Delete",
                            request: DeviceRequest(method: .DELETE, path: "/api/network/connections/\(editor.uuid)"),
                            success: "\(link.name) deleted.",
                            confirm: ActionConfirm(title: "Delete \(link.name)?", text: editor.deleteConfirm, confirm: "Delete")
                        )
                    }
                }
            }
        } else {
            VStack(alignment: .leading, spacing: 0) {
                Text(link.name).font(.bodyLarge)
                Text(linkSummary(link)).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
        }
    }
}

private struct SavedRow: View {
    let profile: SavedProfile
    let methods: [DeviceOption]

    var body: some View {
        DeviceGroup(profile.name, summary: "\(profile.kind) · inactive") {
            RenameForm(uuid: profile.uuid, name: profile.name)
            DeviceForm(
                fields: ipv4Fields(methods, profile.ipv4),
                method: .PUT,
                path: "/api/network/connections/\(profile.uuid)/ipv4",
                submit: "Save",
                success: "\(profile.name) updated.",
                note: "Saved to the profile (not activated)."
            )
            .id(profile.ipv4)
            DeviceButtonRow {
                ActionButton(
                    label: "Activate",
                    request: DeviceRequest(method: .POST, path: "/api/network/connections/\(profile.uuid)/activate"),
                    success: "\(profile.name) activated.",
                    confirm: ActionConfirm(title: "Activate \(profile.name)?", text: profile.activateConfirm, confirm: "Activate")
                )
                DeviceDestructiveButton(
                    label: "Delete",
                    request: DeviceRequest(method: .DELETE, path: "/api/network/connections/\(profile.uuid)"),
                    success: "\(profile.name) deleted.",
                    confirm: ActionConfirm(title: "Delete \(profile.name)?", text: profile.deleteConfirm, confirm: "Delete")
                )
            }
        }
    }
}

private struct DataSection: View {
    let data: DataCard

    var body: some View {
        DeviceSection(title: "Cellular data", badge: data.status) {
            NetSummary(text: data.summary)
            if let gauge = data.gauge {
                DataGaugeCard(gauge: gauge)
            } else {
                DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Neutral, text: "No usage recorded yet — counters accumulate as data flows."))
            }
            DeviceGroup("Data caps & warnings", summary: data.budget.summary) {
                DeviceForm(
                    fields: [
                        DeviceField(name: "monthlyCapGB", label: "Monthly plan (GB)", value: data.budget.monthlyCapGB, placeholder: "e.g. 10 — blank for none"),
                        DeviceField(name: "flightCapMB", label: "Per-flight budget (MB)", value: data.budget.flightCapMB, placeholder: "e.g. 800 — blank for none"),
                        DeviceField(name: "warnPercent", label: "Warn at (%)", value: data.budget.warnPercent, placeholder: "80", keyboard: .number),
                    ],
                    method: .PUT,
                    path: "/api/usage/budget",
                    submit: "Save",
                    success: "Data caps saved.",
                    note: "Warnings alert the ground station; caps never cut the link."
                )
                .id(data.budget)
            }
        }
    }
}

private func percentFraction(_ percent: Int) -> Double { min(max(Double(percent) / 100, 0), 1) }

private struct DataGaugeCard: View {
    let gauge: DataGauge
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        let muted = theme.colors.onSurfaceVariant
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 8) {
                Text("Cellular · LTE").font(.labelMedium).foregroundStyle(muted).frame(maxWidth: .infinity, alignment: .leading)
                ForEach(gauge.badges, id: \.text) { DeviceBadgeText(badge: $0) }
            }
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(gauge.hero).font(.headlineSmall)
                Text(gauge.heroCaption).font(.bodyMedium).foregroundStyle(muted)
            }
            VStack(alignment: .leading, spacing: 6) {
                ProgressView(value: percentFraction(gauge.uplinkPercent)).tint(theme.colors.primary)
                HStack {
                    Text(gauge.uplink).font(.bodySmall).foregroundStyle(muted)
                    Spacer()
                    Text(gauge.downlink).font(.bodySmall).foregroundStyle(muted)
                }
            }
            ForEach(gauge.meters, id: \.label) { BudgetMeterRow(meter: $0) }
            HStack(alignment: .top, spacing: 8) {
                ForEach(gauge.stats, id: \.label) { stat in
                    VStack(alignment: .leading, spacing: 2) {
                        Text(stat.label).font(.labelSmall).foregroundStyle(muted)
                        Text(stat.value).font(deviceMono(.titleSmall))
                        if !stat.note.isEmpty {
                            Text(stat.note).font(.bodySmall).foregroundStyle(muted)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
        .rowPadding(horizontal: 16, vertical: 16)
        .background(nativeList ? Color.clear : theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.medium))
        .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
        .padding(.vertical, nativeList ? Space.s1 : 0)
    }
}

private struct BudgetMeterRow: View {
    let meter: BudgetMeter
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 8) {
                Text(meter.label).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).frame(maxWidth: .infinity, alignment: .leading)
                Text(meter.used).font(deviceMono(.bodySmall)).foregroundStyle(neutralOr(meter.level, theme.colors.onSurface, theme))
            }
            ProgressView(value: percentFraction(meter.percent)).tint(neutralOr(meter.level, theme.colors.primary, theme))
            Text(meter.remaining).font(.bodySmall).foregroundStyle(deviceLevelColor(meter.level, theme))
            if !meter.note.isEmpty {
                Text(meter.note).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
    }
}

struct TailscaleSection: View {
    let tailscale: TailscaleCard
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList

    private var hasActions: Bool { !tailscale.connect.isEmpty || tailscale.cancel || !tailscale.logOut.isEmpty }

    var body: some View {
        DeviceSection(title: "Tailscale", badge: tailscale.status) {
            NetSummary(text: tailscale.summary)
            if let problem = tailscale.problem { ProblemNotice(problem: problem) }
            if let banner = tailscale.banner {
                HStack(spacing: 12) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(banner.title).font(.titleSmall)
                        Text(banner.subtitle).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    DeviceBadgeText(badge: banner.status)
                }
                .rowPadding(horizontal: 16, vertical: 12)
                .background(nativeList ? Color.clear : theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.medium))
                .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
            }
            ForEach(tailscale.facts, id: \.label) { DeviceFact(label: $0.label, value: $0.value) }
            if !tailscale.addresses.isEmpty { DeviceFact(label: "This device's addresses", value: tailscale.addresses.joined(separator: ", ")) }
            if let signIn = tailscale.signIn {
                VStack(alignment: .leading, spacing: 8) {
                    LinkButton(href: signIn.href, text: signIn.label, external: true)
                    Text(signIn.href).font(deviceMono(.bodySmall)).foregroundStyle(theme.colors.onSurfaceVariant).textSelection(.enabled)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
            }
            if hasActions {
                DeviceButtonRow {
                    if !tailscale.connect.isEmpty {
                        ActionButton(label: tailscale.connect, request: DeviceRequest(method: .POST, path: "/api/tailscale/connection"))
                    }
                    if tailscale.cancel {
                        ActionButton(
                            label: "Cancel sign-in",
                            request: DeviceRequest(method: .POST, path: "/api/tailscale/connection/cancel"),
                            success: "Sign-in cancelled.",
                            tonal: false
                        )
                    }
                    if !tailscale.logOut.isEmpty {
                        ActionButton(
                            label: "Log out",
                            request: DeviceRequest(method: .DELETE, path: "/api/tailscale/connection"),
                            success: "Logged out of Tailscale.",
                            confirm: ActionConfirm(title: "Log out of the tailnet?", text: tailscale.logOut, confirm: "Log out")
                        )
                    }
                }
            }
            DeviceText(text: tailscale.hint)
            if let control = tailscale.control { ControlServer(control: control) }
        }
    }
}

private struct ControlServer: View {
    let control: TailscaleControl

    var body: some View {
        DeviceGroup(control.title, initiallyOpen: control.open) {
            DeviceForm(
                fields: [
                    DeviceField(name: "controlServer", label: "Control server URL", value: control.server, placeholder: "https://headscale.example.com", keyboard: .uri),
                    DeviceField(
                        name: "authKey",
                        label: "Auth key — device joins automatically, no sign-in",
                        value: "",
                        placeholder: control.authKeyPlaceholder,
                        keyboard: .password
                    ),
                ],
                method: .PUT,
                path: "/api/tailscale/control",
                submit: "Save",
                success: "Control server saved."
            )
            .id("\(control.server)|\(control.authKeySet)")
            if control.authKeySet {
                DeviceButtonRow {
                    DeviceDestructiveButton(
                        label: "Remove auth key",
                        request: DeviceRequest(method: .DELETE, path: "/api/tailscale/control/authkey"),
                        success: "Auth key removed.",
                        confirm: ActionConfirm(
                            title: "Remove the auth key?",
                            text: "Remove the saved auth key? Switching will then need an interactive sign-in over the LAN.",
                            confirm: "Remove"
                        )
                    )
                }
            }
            if !control.switchError.isEmpty {
                DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Blocked, text: "Last switch failed (rolled back): \(control.switchError)"))
            }
            DeviceText(text: control.pending)
            if control.switchTo != nil || control.logOutToSwitch != nil {
                DeviceButtonRow {
                    if let switchTo = control.switchTo {
                        ActionButton(
                            label: switchTo.label,
                            request: DeviceRequest(method: .POST, path: "/api/tailscale/switch"),
                            success: "Switching… if it fails it rolls back automatically.",
                            confirm: ActionConfirm(title: "\(switchTo.label)?", text: switchTo.confirm, confirm: "Switch")
                        )
                    }
                    if let logOut = control.logOutToSwitch {
                        ActionButton(
                            label: logOut.label,
                            request: DeviceRequest(method: .DELETE, path: "/api/tailscale/connection"),
                            success: "Logged out — now connect to finish switching.",
                            confirm: ActionConfirm(title: "Log out to switch?", text: logOut.confirm, confirm: "Log out")
                        )
                    }
                }
            }
            DeviceText(text: control.hint)
        }
    }
}

private func relayMode(_ relay: Bool) -> DeviceRequest {
    DeviceRequest(method: .PUT, path: "/api/stream/webrtc-mode", body: deviceBody(["relay": relay ? "true" : "false"]))
}

private struct RelaySection: View {
    let relay: RelayCard

    var body: some View {
        DeviceSection(title: "Relay server (TURN)", badge: relay.status) {
            NetSummary(text: relay.summary)
            DeviceSwitchRow(
                title: "Use my relay instead of Tailscale",
                checked: relay.configured && relay.on,
                on: relayMode(true),
                off: relayMode(false),
                detail: relay.configured ? "" : relay.reason,
                success: "Remote video now goes through your relay.",
                offSuccess: "Remote video goes over Tailscale again.",
                enabled: relay.configured
            )
            if !relay.notice.text.isEmpty { DeviceNotice(notice: relay.notice) }
            IceEditor(servers: relay.servers).id(relay.servers)
            DeviceGroup("Technical details") {
                DeviceText(
                    text: "Off (default) advertises the tailscale 100.x and LAN host candidates. On stops advertising the tailscale host candidate and routes WebRTC media through the configured STUN/TURN ICE servers over UDP, so symmetric-CGNAT sessions on cellular relay over UDP instead of falling back to Tailscale DERP (TCP). Same-LAN viewers still use the direct LAN candidate."
                )
            }
        }
    }
}

private struct IceEditor: View {
    @Environment(\.theme) private var theme
    @Environment(\.LocalChangeNotice) private var notice
    @State private var rows: [IceServer]
    @State private var action = DeviceActionState()

    init(servers: [IceServer]) {
        _rows = State(initialValue: iceRows(servers))
    }

    private func change(_ index: Int, _ next: IceServer) {
        rows = rows.enumerated().map { at, row in at == index ? next : row }
    }

    private func remove(_ index: Int) {
        rows = rows.enumerated().filter { $0.offset != index }.map(\.element)
    }

    private func save() {
        let sent = rows, action = action, notice = notice
        Task { @MainActor in
            guard let servers = await iceServerLines(sent) else { return }
            _ = await action.run(
                DeviceRequest(method: .PUT, path: "/api/stream/webrtc-ice-servers", body: deviceBody(["servers": servers])),
                success: "Servers saved.",
                notice: notice
            )
        }
    }

    // Rows of the relay section: one per server (URL, username, password and a red minus), the add buttons, Save.
    var body: some View {
        Text("Relay (TURN / STUN) servers").font(.headline)
        ForEach(Array(rows.enumerated()), id: \.offset) { index, row in
            VStack(alignment: .leading, spacing: Space.s2) {
                HStack(spacing: Space.s2) {
                    DeviceTextField(
                        label: "Server URL",
                        value: row.url,
                        onValueChange: { url in change(index, withChanges(row) { $0.url = url }) },
                        placeholder: "turn:turn.example.com:443?transport=udp",
                        keyboard: .uri
                    )
                    DeviceRemoveButton(label: "Remove server") { remove(index) }
                }
                // .password is the plain keyboard without autocapitalisation or autocorrect, which a username needs.
                DeviceTextField(
                    label: "Username",
                    value: row.username,
                    onValueChange: { username in change(index, withChanges(row) { $0.username = username }) },
                    placeholder: "username",
                    keyboard: .password
                )
                SecretField(
                    label: "Password",
                    value: row.password,
                    onValueChange: { password in change(index, withChanges(row) { $0.password = password }) },
                    placeholder: "password"
                )
            }
        }
        HStack(spacing: Space.s4) {
            Button {
                rows.append(IceServer())
            } label: {
                Label("Add server", systemImage: "plus.circle.fill")
            }
            Button {
                rows.append(IceServer(url: GOOGLE_STUN))
            } label: {
                Label("Add Google STUN", systemImage: "plus.circle.fill")
            }
        }
        .buttonStyle(.borderless)
        if let refusal = action.refusal {
            Text(refusal.message).font(.footnote).foregroundStyle(theme.colors.error)
        }
        VStack(alignment: .leading, spacing: Space.s2) {
            Button("Save servers", action: save).buttonStyle(.bordered).disabled(action.busy)
            Text("A TURN server relays video when a direct connection isn't possible — use one on UDP, ideally port 443. STUN-only lines need no username or password.")
                .font(.footnote)
                .foregroundStyle(Color.secondary)
        }
    }
}
