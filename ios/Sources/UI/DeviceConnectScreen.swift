import SwiftUI

struct DeviceConnectScreen: View {
    let props: DeviceScreenProps
    @DeviceView("connect") private var reading
    @State private var saved: String? = readStored(GCS_KEY)

    private func pick(_ value: String) {
        writeStored(GCS_KEY, value)
        saved = value
    }

    var body: some View {
        if let view = deviceView(reading, parseConnect) {
            if let add = view.noCamera {
                NoCamera(add: add, onOpen: props.onOpen)
            } else {
                ConnectLinks(view: view, choice: chosenGcs(view.choices, saved), onPick: pick, onOpen: props.onOpen)
            }
        } else {
            FootNote(text: DEVICE_WAITING)
        }
    }
}

private struct NoCamera: View {
    let add: PageLink
    let onOpen: (String) -> Void

    var body: some View {
        DeviceSection(title: "Cameras") {
            DeviceText(text: "There is no camera on this device yet, so there is nothing to point a ground station at.")
            Button(add.label) { onOpen(add.href) }
                .buttonStyle(.bordered)
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
        }
    }
}

private struct ReachLine: View {
    let reach: Bool
    @Environment(\.theme) private var theme

    var body: some View {
        Text(reach ? "This address answers from here" : "No answer from here — your ground station may not reach it either")
            .font(.labelMedium)
            .foregroundStyle(reach ? theme.aircast.success : theme.aircast.warning)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(EdgeInsets(top: 0, leading: 16, bottom: 8, trailing: 16))
    }
}

private struct TargetRow: View {
    let target: ConnectTarget
    @Environment(\.theme) private var theme
    @State private var reach: Bool?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            CopyRow(
                label: target.tag.isEmpty ? target.label : "\(target.label) · \(target.tag)",
                text: target.url,
                hint: target.hint,
                qr: target.qr,
                level: target.down ? DeviceLevel.Warn : DeviceLevel.Neutral
            )
            if let reach { ReachLine(reach: reach) }
            if !target.alternatives.isEmpty {
                VStack(alignment: .leading, spacing: 0) {
                    DeviceText(text: "Doesn't resolve on Android or Windows? Use the address instead:")
                    ForEach(target.alternatives, id: \.url) { CopyRow(label: $0.label, text: $0.url) }
                }
                .overlay(alignment: .leading) { Rectangle().fill(theme.colors.outlineVariant).frame(width: 2) }
                .padding(.leading, 16)
            }
        }
        .task(id: target.reach) {
            let url = target.reach
            reach = nil
            guard !probeBlocked(url) else { return }
            while !Task.isCancelled {
                reach = await deviceReach(url)
                try? await Task.sleep(for: .milliseconds(REACH_RECHECK_MS))
            }
        }
    }
}

private struct TargetRows: View {
    let targets: [ConnectTarget]
    let choice: GcsChoice?

    var body: some View {
        ForEach(shownTargets(targets, choice), id: \.url) { TargetRow(target: $0) }
    }
}

private struct GcsChooser: View {
    let choices: [GcsChoice]
    let choice: GcsChoice?
    let onPick: (String) -> Void

    var body: some View {
        DeviceButtonRow {
            ForEach(choices, id: \.value) { option in
                DeviceChip(label: option.label, selected: option.value == choice?.value) { onPick(option.value) }
            }
        }
    }
}

private struct Subheading: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.titleSmall)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(EdgeInsets(top: 16, leading: 16, bottom: 4, trailing: 16))
    }
}

private struct ConnectLinks: View {
    let view: ConnectView
    let choice: GcsChoice?
    let onPick: (String) -> Void
    let onOpen: (String) -> Void

    var body: some View {
        DeviceSection(title: "Your ground station") {
            GcsChooser(choices: view.choices, choice: choice, onPick: onPick)
            DeviceText(text: "QGroundControl and Mission Planner play the RTSP link; a browser, OBS or Aircast QGC play WebRTC. This app remembers your answer.")
            DeviceText(text: view.scope)
            DeviceDisclosure("How to connect a ground station") {
                DeviceText(text: "Each link below says where to paste it in your app; video and telemetry are two separate links.")
                DeviceText(text: "Ground station on another device? Scan that link's QR code from there — no retyping.")
            }
        }
        ForEach(view.cameras, id: \.name) { camera in
            DeviceSection(title: camera.name, badge: camera.status) {
                if !camera.notice.isEmpty { DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Warn, text: camera.notice)) }
                if let fix = camera.fix {
                    Button(fix.label) { onOpen(fix.href) }
                        .buttonStyle(.bordered)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 8)
                }
                TargetRows(targets: camera.targets, choice: choice)
            }
        }
        if let telemetry = view.telemetry {
            DeviceSection(title: "Telemetry", badge: telemetry.status) {
                DeviceText(text: "Video and telemetry are separate links — a ground station needs both.")
                TargetRows(targets: telemetry.targets, choice: choice)
            }
        }
        if let trouble = view.trouble { TroubleSection(trouble: trouble, choice: choice) }
    }
}

private struct TroubleSection: View {
    let trouble: ConnectTrouble
    let choice: GcsChoice?
    @Environment(\.theme) private var theme

    var body: some View {
        DeviceSection(title: "Troubleshooting") {
            DeviceDisclosure("This link didn't play in my app", summary: "Reach the device by address, or fall back to a player that takes anything.") {
                if !trouble.fallback.isEmpty {
                    Subheading(text: "When nothing else will play")
                    TargetRows(targets: trouble.fallback, choice: choice)
                }
                if let addresses = trouble.addresses {
                    Subheading(text: addresses.title)
                    DeviceText(
                        text: "Windows without Bonjour, and some networks, cannot look up a .local name — and a tailnet name needs Tailscale running there too. " +
                            "These links use this device's address instead: pick the one on the same network as your ground station."
                    )
                    ForEach(addresses.groups, id: \.label) { group in
                        Text(group.label)
                            .font(.labelMedium)
                            .foregroundStyle(theme.colors.onSurfaceVariant)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(EdgeInsets(top: 12, leading: 16, bottom: 0, trailing: 16))
                        TargetRows(targets: group.targets, choice: choice)
                    }
                }
            }
        }
    }
}
