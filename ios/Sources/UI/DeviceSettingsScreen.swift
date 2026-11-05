import SwiftUI

struct DeviceSettingsScreen: View {
    let props: DeviceScreenProps
    @DeviceView("device") private var reading

    var body: some View {
        NativeForm {
            if let view = deviceView(reading, parseDeviceSettings) {
                Section {
                    HealthLine(health: view.health, onOpen: props.onOpen)
                    if let notice = view.notice { DeviceNotice(notice: notice) }
                }
                DeviceSection(title: "Status") {
                    ForEach(view.stats, id: \.label) { stat in
                        if stat.label == SERVICES {
                            ServicesStat(stat: stat, services: view.services)
                        } else {
                            StatText(stat: stat)
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
}

private struct HealthLine: View {
    let health: HealthBadge
    let onOpen: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: Space.s3) {
            Circle().fill(deviceLevelColor(health.level, theme)).frame(width: 10, height: 10)
            Text(health.text).font(.headline).foregroundStyle(deviceLevelColor(health.level, theme)).frame(maxWidth: .infinity, alignment: .leading)
            if !health.href.isEmpty {
                Button("Check") { onOpen(health.href) }.buttonStyle(.bordered)
            }
        }
    }
}

private struct StatText: View {
    let stat: DeviceStat
    @Environment(\.theme) private var theme

    // A value row: the label left, the value right (coloured by its level), the hint underneath.
    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            LabeledContent(stat.label) {
                Text(stat.value)
                    .foregroundStyle(stat.level == DeviceLevel.Neutral ? Color.secondary : deviceLevelColor(stat.level, theme))
                    .multilineTextAlignment(.trailing)
            }
            if !stat.hint.isEmpty {
                Text(stat.hint).font(.footnote).foregroundStyle(Color.secondary)
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

    // The services stat opens to one row per supervised unit.
    var body: some View {
        DisclosureGroup(isExpanded: $open) {
            ForEach(services, id: \.label) { service in
                LabeledContent(service.label) {
                    Text(service.state).foregroundStyle(deviceLevelColor(service.level, theme))
                }
            }
            DeviceStatusLine(text: "Supervised by systemd — a unit that stops is restarted automatically.")
        } label: {
            StatText(stat: stat)
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
        DeviceGroup("Advanced · environment", summary: "Which Aircast cloud to sign in to", initiallyOpen: refusedIssuer) {
            DeviceTextField(
                label: "Cloud API",
                value: issuer,
                onValueChange: { issuer = $0 },
                placeholder: "https://api.aircast.one",
                isError: refusedIssuer,
                supporting: "Leave this alone unless you were told to use a different Aircast environment.",
                keyboard: .uri
            )
        }
        DeviceRefusalText(refusal: action.refusal, field: action.refusal?.field ?? "")
        Button("Connect", action: connect).buttonStyle(.borderedProminent).disabled(action.busy)
    }
}

private struct SoftwareNoticeRow: View {
    let notice: SoftwareNotice

    var body: some View {
        DeviceNotice(notice: DeviceBadge(level: notice.level, text: notice.text.isEmpty ? notice.title : "\(notice.title) \(notice.text)"))
        if !notice.detail.isEmpty {
            DeviceGroup(notice.detailLabel) {
                ScrollView {
                    Text(notice.detail).font(deviceMono(.bodySmall)).frame(maxWidth: .infinity, alignment: .leading).textSelection(.enabled)
                }
                .frame(maxHeight: 192)
            }
        }
    }
}

private struct SoftwareSection: View {
    let software: SoftwareStatus

    var body: some View {
        DeviceSection(title: "Software", badge: software.status) {
            DeviceFact(label: "Running", value: software.summary)
            ForEach(software.notices, id: \.title) { SoftwareNoticeRow(notice: $0) }
            if let candidate = software.candidate {
                VStack(alignment: .leading, spacing: Space.s2) {
                    Text(candidate.title).font(.headline)
                    HStack(spacing: Space.s2) {
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
                .padding(.vertical, Space.s1)
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Available update")
            }
            ForEach(software.notes, id: \.self) { DeviceText(text: $0) }
            HStack(spacing: Space.s2) {
                Text(software.checked).font(.footnote).foregroundStyle(Color.secondary).frame(maxWidth: .infinity, alignment: .leading)
                if software.canCheck {
                    ActionButton(label: "Check now", request: DeviceRequest(method: .POST, path: "/api/update/check"), success: "Checked for updates.", tonal: false)
                }
            }
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
    @Environment(\.LocalDeviceOrigin) private var origin

    var body: some View {
        DeviceGroup("Set up another device like this one", summary: "Export these settings for the next card.") {
            ForEach(Array(PROVISION_STEPS.enumerated()), id: \.offset) { index, step in
                HStack(alignment: .top, spacing: Space.s2) {
                    Text("\(index + 1).").foregroundStyle(Color.secondary)
                    Text(step).frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            DeviceText(text: "The file carries the camera's own address, sign-in included — that is what makes it worth copying. Treat it like the password it contains.")
            DeviceButtonRow { LinkButton(href: resolveDeviceUrl(origin, "/api/provision/export"), text: "Export these settings", icon: .download) }
            DeviceStatusLine(text: "Airframes two through five are this one with a different serial.")
        }
    }
}

private struct ResetSection: View {
    let hostname: String
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

    // A destructive row in its own section, the explanation as the footer; the typed confirmation is unchanged.
    var body: some View {
        Section {
            Button("Reset to defaults", role: .destructive) {
                typed = ""
                asking = true
            }
            .disabled(action.busy)
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
            DeviceRefusalText(refusal: action.refusal, field: action.refusal?.field ?? "")
        } header: {
            SectionHeader(text: "Reset device")
        } footer: {
            Text("Reboots and reverts the device to factory settings — camera, telemetry and all Wi-Fi and added network settings are wiped, and the setup wizard runs again. Tailscale stays connected so the device remains reachable.")
        }
    }
}
