import SwiftUI

struct DeviceSettingsScreen: View {
    let props: DeviceScreenProps
    @DeviceView("device") private var reading

    var body: some View {
        if let view = deviceView(reading, parseDeviceSettings) {
            HealthLine(health: view.health, onOpen: props.onOpen)
            if let notice = view.notice { DeviceNotice(notice: notice) }
            DeviceSection(title: "Status") {
                ForEach(view.stats, id: \.label) { stat in
                    if stat.label == SERVICES {
                        ServicesStat(stat: stat, services: view.services)
                    } else {
                        StatText(stat: stat).padding(.horizontal, 16).padding(.vertical, 8)
                    }
                }
            }
            CloudSection(cloud: view.cloud)
            SoftwareSection(software: view.software)
            DeviceSection(title: "More devices") { ProvisionSteps() }
            ResetSection(hostname: view.hostname)
        } else {
            FootNote(text: DEVICE_WAITING)
        }
    }
}

private struct HealthLine: View {
    let health: HealthBadge
    let onOpen: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: 12) {
            Circle().fill(deviceLevelColor(health.level, theme)).frame(width: 10, height: 10)
            Text(health.text).font(.titleMedium).foregroundStyle(deviceLevelColor(health.level, theme)).frame(maxWidth: .infinity, alignment: .leading)
            if !health.href.isEmpty {
                Button("Check") { onOpen(health.href) }.buttonStyle(.bordered)
            }
        }
        .padding(EdgeInsets(top: 16, leading: 16, bottom: 4, trailing: 16))
    }
}

private struct StatText: View {
    let stat: DeviceStat
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(stat.label).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            Text(stat.value).font(.titleMedium).foregroundStyle(deviceLevelTextColor(stat.level, theme))
            if !stat.hint.isEmpty {
                Text(stat.hint).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct ServicesStat: View {
    let stat: DeviceStat
    let services: [ServiceRow]
    @Environment(\.theme) private var theme
    @State private var open = false

    var body: some View {
        Button { open.toggle() } label: {
            HStack(spacing: 16) {
                StatText(stat: stat)
                Text(open ? "Hide" : "Show details").font(.labelMedium).foregroundStyle(theme.colors.primary)
                Image(open ? .arrowUp : .arrowDropDown).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        if open {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(services, id: \.label) { service in
                    HStack(spacing: 12) {
                        Text(service.label).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                        Text(service.state).font(.labelLarge).foregroundStyle(deviceLevelColor(service.level, theme))
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 6)
                }
                DeviceStatusLine(text: "Supervised by systemd — a unit that stops is restarted automatically.")
            }
            .padding(.bottom, 8)
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Supervised units")
        }
    }
}

private struct CloudSection: View {
    let cloud: CloudAccount

    var body: some View {
        DeviceSection(title: "Cloud account", badge: cloud.status) {
            switch cloud.stage {
            case CloudStage.Linked:
                DeviceText(text: "This device is linked to your Aircast cloud account. The ID below only matters if support asks for it.")
                CopyRow(label: "Device ID (for support)", text: cloud.deviceId)
                if !cloud.watchUrl.isEmpty {
                    DeviceButtonRow { LinkButton(href: cloud.watchUrl, text: "Watch in the cloud", look: .Text, external: true) }
                }
            case CloudStage.SigningIn:
                DeviceText(text: "Open the sign-in page, log in (or sign up) with Google, and authorize this device. It connects automatically once you approve.")
                DeviceButtonRow { LinkButton(href: cloud.signInUrl, text: "Open sign-in page", external: true) }
                CopyRow(label: "Or go to the link and enter this code", text: cloud.userCode)
            default:
                CloudConnect(cloud: cloud).id(cloud.issuer)
            }
        }
    }
}

private struct CloudConnect: View {
    let cloud: CloudAccount
    @Environment(\.LocalChangeNotice) private var notice
    @State private var action = DeviceActionState()
    @State private var issuer: String

    init(cloud: CloudAccount) {
        self.cloud = cloud
        _issuer = State(initialValue: cloud.issuer)
    }

    private func connect() {
        let request = DeviceRequest(method: .POST, path: "/api/cloud/connect", body: deviceBody(["issuer": issuer]))
        let notice = notice, action = action
        Task { @MainActor in _ = await action.run(request, success: "Sign-in started — the code appears here in a moment.", notice: notice) }
    }

    var body: some View {
        let refusedIssuer = action.refusal?.field == "issuer"
        DeviceText(text: "Not connected. Linking signs this device in to an Aircast cloud account; it is optional — every local and self-hosted path works without it.")
        if !cloud.error.isEmpty { DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Warn, text: cloud.error)) }
        DeviceDisclosure("Advanced · environment", summary: "Which Aircast cloud to sign in to", initiallyOpen: refusedIssuer) {
            DeviceTextField(
                label: "Cloud API",
                value: issuer,
                onValueChange: { issuer = $0 },
                placeholder: "https://api.aircast.one",
                isError: refusedIssuer,
                supporting: "Leave this alone unless you were told to use a different Aircast environment.",
                keyboard: .uri
            )
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
        }
        DeviceRefusalText(refusal: action.refusal, field: action.refusal?.field ?? "")
        DeviceButtonRow {
            Button("Connect", action: connect).buttonStyle(.bordered).disabled(action.busy)
        }
    }
}

private struct SoftwareNoticeRow: View {
    let notice: SoftwareNotice
    @Environment(\.theme) private var theme

    var body: some View {
        DeviceNotice(notice: DeviceBadge(level: notice.level, text: notice.text.isEmpty ? notice.title : "\(notice.title) \(notice.text)"))
        if !notice.detail.isEmpty {
            DeviceDisclosure(notice.detailLabel) {
                ScrollView {
                    Text(notice.detail).font(deviceMono(.bodySmall)).frame(maxWidth: .infinity, alignment: .leading).textSelection(.enabled)
                }
                .frame(maxHeight: 192)
                .padding(12)
                .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.small))
                .padding(EdgeInsets(top: 0, leading: 16, bottom: 8, trailing: 16))
            }
        }
    }
}

private struct SoftwareSection: View {
    let software: SoftwareStatus
    @Environment(\.theme) private var theme

    var body: some View {
        DeviceSection(title: "Software", badge: software.status) {
            StackedFact(label: "Running", value: software.summary)
            ForEach(software.notices, id: \.title) { SoftwareNoticeRow(notice: $0) }
            if let candidate = software.candidate {
                VStack(alignment: .leading, spacing: 8) {
                    Text(candidate.title).font(.titleMedium)
                    HStack(spacing: 8) {
                        ActionButton(
                            label: candidate.install,
                            request: DeviceRequest(method: .POST, path: "/api/update"),
                            success: candidate.success,
                            confirm: ActionConfirm(title: "Install the update?", text: candidate.confirm, confirm: "Install")
                        )
                        LinkButton(href: candidate.notesUrl, text: "What's new", look: .Text, external: true)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(16)
                .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.medium))
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Available update")
            }
            ForEach(software.notes, id: \.self) { DeviceText(text: $0) }
            HStack(spacing: 8) {
                Text(software.checked).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).frame(maxWidth: .infinity, alignment: .leading)
                if software.canCheck {
                    ActionButton(label: "Check now", request: DeviceRequest(method: .POST, path: "/api/update/check"), success: "Checked for updates.", tonal: false)
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
        }
    }
}

private let PROVISION_STEPS = [
    "Export this device's settings.",
    "Add your tailnet auth key to the file. It is not exported, because this page answers anyone who can reach the device.",
    "Copy the file onto the new card's boot partition as aircast-provision.json.",
    "Boot it. It joins, picks up the camera and the data caps, then deletes the file so the key does not stay on the card.",
]

private struct ProvisionSteps: View {
    @Environment(\.theme) private var theme
    @Environment(\.LocalDeviceOrigin) private var origin

    var body: some View {
        DeviceDisclosure("Set up another device like this one", summary: "Export these settings for the next card.") {
            ForEach(Array(PROVISION_STEPS.enumerated()), id: \.offset) { index, step in
                HStack(alignment: .top, spacing: 8) {
                    Text("\(index + 1).").font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                    Text(step).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant).frame(maxWidth: .infinity, alignment: .leading)
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 4)
            }
            DeviceText(text: "The file carries the camera's own address, sign-in included — that is what makes it worth copying. Treat it like the password it contains.")
            DeviceButtonRow { LinkButton(href: resolveDeviceUrl(origin, "/api/provision/export"), text: "Export these settings", icon: .download) }
            DeviceStatusLine(text: "Airframes two through five are this one with a different serial.")
        }
    }
}

private struct ResetSection: View {
    let hostname: String
    @Environment(\.theme) private var theme
    @Environment(\.LocalChangeNotice) private var notice
    @State private var action = DeviceActionState()
    @State private var asking = false
    @State private var typed = ""

    private func reset(_ confirm: String) {
        let request = DeviceRequest(method: .POST, path: "/api/setup/reset", body: deviceBody(["confirm": confirm]))
        let notice = notice, action = action
        Task { @MainActor in
            _ = await action.run(request, success: "Resetting — the device is rebooting to factory settings; reconnect in a minute.", notice: notice)
        }
    }

    var body: some View {
        DeviceSection(title: "Reset device") {
            DeviceText(text: "Reboots and reverts the device to factory settings — camera, telemetry and all Wi-Fi and added network settings are wiped, and the setup wizard runs again. Tailscale stays connected so the device remains reachable.")
            DeviceButtonRow {
                Button("Reset to defaults") {
                    typed = ""
                    asking = true
                }
                .buttonStyle(.bordered)
                .tint(theme.colors.error)
                .disabled(action.busy)
            }
            DeviceRefusalText(refusal: action.refusal, field: action.refusal?.field ?? "")
        }
        .deviceTypedConfirm(
            $asking,
            title: "Reset to factory settings?",
            text: "It reboots and wipes the camera, telemetry and all Wi-Fi and added network settings, then runs setup again. Tailscale is preserved.",
            label: "Type \(hostname) to confirm",
            expected: hostname,
            confirm: "Reset",
            typed: $typed,
            onConfirm: reset
        )
    }
}
