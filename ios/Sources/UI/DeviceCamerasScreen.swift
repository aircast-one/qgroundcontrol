import SwiftUI

struct DeviceCamerasScreen: View {
    let props: DeviceScreenProps
    @DeviceView("cameras") private var reading
    @Environment(\.LocalChangeNotice) private var notice
    @State private var picked: String
    @State private var adding: Bool
    @State private var seen: [String: Bool] = [:]

    init(props: DeviceScreenProps) {
        self.props = props
        _picked = State(initialValue: props.query["camera"] ?? "")
        _adding = State(initialValue: props.query["add"] != nil)
    }

    private func pick(_ name: String) {
        adding = false
        picked = name
    }

    private func announceComebacks(_ cameras: [CameraView]?) {
        guard let cameras else { return }
        if cameras.isEmpty { adding = true }
        let back = cameBack(seen, cameras)
        if !back.isEmpty { notice.show("\(back.joined(separator: ", ")) \(back.count == 1 ? "is" : "are") back — sending video.", nil) }
        seen = liveness(cameras)
    }

    var body: some View {
        let view = deviceView(reading, parseCameras)
        VStack(alignment: .leading, spacing: 0) {
            if let view {
                CamerasBody(
                    view: view,
                    shown: camerasShown(view.cameras, picked, adding),
                    props: props,
                    onAdd: { adding = true },
                    onPick: { pick($0) }
                )
            } else {
                FootNote(text: DEVICE_WAITING)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .onChange(of: view?.cameras, initial: true) { _, cameras in announceComebacks(cameras) }
    }
}

private func shownCamera(_ shown: CamerasShown) -> CameraView? {
    if case .Camera(let camera) = shown { return camera }
    return nil
}

private struct CamerasBody: View {
    let view: CamerasView
    let shown: CamerasShown
    let props: DeviceScreenProps
    let onAdd: () -> Void
    let onPick: (String) -> Void

    var body: some View {
        let selected = shownCamera(shown)
        let canAdd = shown != .Add
        let fixing = selected?.live == false
        if !view.cameras.isEmpty {
            CameraTabs(cameras: view.cameras, shown: selected, onPick: onPick)
        }
        if !fixing && (canAdd || !view.cloudLink.isEmpty) {
            HStack(spacing: 8) {
                if canAdd { Button("+ Add camera", action: onAdd).buttonStyle(.text) }
                Spacer(minLength: 0)
                if !view.cloudLink.isEmpty { LinkButton(href: view.cloudLink, text: "Watch in the cloud", look: .Outlined, external: true) }
            }
            .padding(EdgeInsets(top: 8, leading: 16, bottom: 0, trailing: 16))
        }
        switch shown {
        case .Camera(let camera):
            CameraDetail(camera: camera, scan: props.query["scan"] != nil && props.query["camera"] == camera.name, onOpen: props.onOpen)
                .id(camera.name)
            TopicRow(page: camera.topic, onOpen: props.onOpen)
            TopicRow(page: view.video, onOpen: props.onOpen)
        case .Wall:
            ForEach(view.cameras, id: \.name) { camera in
                CameraTile(camera: camera) { onPick(camera.name) }
            }
            TopicRow(page: view.video, onOpen: props.onOpen)
        case .Add:
            DeviceSection(title: view.add.title) {
                CameraWizard(view: view.add, onDone: onPick)
            }
        }
    }
}

private struct CameraTabs: View {
    let cameras: [CameraView]
    let shown: CameraView?
    let onPick: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 0) {
                ForEach(cameras, id: \.name) { camera in
                    let selected = camera == shown
                    Button { onPick(camera.name) } label: {
                        VStack(spacing: 0) {
                            HStack(spacing: 8) {
                                Circle().fill(deviceLevelColor(camera.status.level, theme)).frame(width: 8, height: 8)
                                Text(camera.name).font(.titleSmall).foregroundStyle(selected ? theme.colors.primary : theme.colors.onSurface)
                                Text(camera.status.text).font(.labelSmall).foregroundStyle(deviceLevelColor(camera.status.level, theme))
                            }
                            .padding(.horizontal, 16)
                            .frame(minHeight: 48)
                            Rectangle().fill(selected ? theme.colors.primary : Color.clear).frame(height: 3)
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(selected ? .isSelected : [])
                }
            }
        }
        .overlay(alignment: .bottom) { Rectangle().fill(theme.colors.outlineVariant).frame(height: 1) }
    }
}

private struct TopicRow: View {
    let page: DevicePage
    let onOpen: (String) -> Void

    var body: some View {
        SetupRow(
            title: page.title,
            state: page.needs ? .NeedsAttention : .Neutral,
            onClick: { onOpen(page.href) },
            icon: devicePageIcon(page.icon),
            subtitle: page.summary
        )
    }
}

private struct CameraPicture: View {
    let camera: CameraView
    @Environment(\.LocalDeviceOrigin) private var origin

    var body: some View {
        if camera.preview.isEmpty {
            PreviewPlaceholder(text: camera.placeholder)
        } else {
            WhepPreview(endpoint: resolveDeviceUrl(origin, camera.preview), watch: resolveDeviceUrl(origin, camera.watch), name: camera.name)
        }
    }
}

private struct CameraTile: View {
    let camera: CameraView
    let onPick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 12) {
                    Button(action: onPick) { Text(camera.name).font(.titleMedium).foregroundStyle(theme.colors.onSurface) }
                        .buttonStyle(.plain)
                    DeviceBadgeText(badge: camera.status)
                    Spacer(minLength: 0)
                    Button("Open", action: onPick).buttonStyle(.bordered)
                }
                CameraPicture(camera: camera)
                if let diagnosis = camera.diagnosis {
                    Text(diagnosis.text).font(.bodyMedium).foregroundStyle(deviceLevelColor(diagnosis.level, theme))
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            DeviceSwitchRow(
                title: "Record to disk",
                checked: camera.recording,
                on: DeviceRequest(method: .PUT, path: "/api/streams/\(camera.name)/record"),
                off: DeviceRequest(method: .DELETE, path: "/api/streams/\(camera.name)/record"),
                success: "Recording turned on for \(camera.name).",
                offSuccess: "Recording turned off for \(camera.name) — download it under Settings → Recordings."
            )
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(camera.name)
    }
}

private struct CameraDetail: View {
    let camera: CameraView
    let onOpen: (String) -> Void
    @State private var wizard: Bool?

    init(camera: CameraView, scan: Bool, onOpen: @escaping (String) -> Void) {
        self.camera = camera
        self.onOpen = onOpen
        _wizard = State(initialValue: scan ? true : nil)
    }

    var body: some View {
        if let scanning = wizard {
            DeviceSection(title: camera.wizard.title, badge: camera.status) {
                CameraWizard(view: camera.wizard, scan: scanning, onCancel: { wizard = nil }, onDone: { _ in wizard = nil })
            }
        } else {
            CameraOverview(camera: camera, onOpen: onOpen) { wizard = $0 }
        }
    }
}

private struct CameraOverview: View {
    let camera: CameraView
    let onOpen: (String) -> Void
    let onWizard: (Bool) -> Void
    @Environment(\.theme) private var theme
    @Environment(\.LocalDeviceOrigin) private var origin

    var body: some View {
        DeviceSection(title: camera.name, badge: camera.status) {
            if camera.shared {
                DeviceBadgeText(badge: DeviceBadge(level: DeviceLevel.Warn, text: "shared source")).padding(.horizontal, 16)
                DeviceNotice(
                    notice: DeviceBadge(
                        level: DeviceLevel.Warn,
                        text: "Another camera pulls this same RTSP source. One camera fed to two pulls splits its video and audio across the streams — remove the duplicate camera for a stable feed."
                    )
                )
            }
            CameraPicture(camera: camera)
                .frame(maxWidth: 576)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
            if camera.live {
                DeviceButtonRow {
                    LinkButton(href: resolveDeviceUrl(origin, camera.watch), text: "▶ Watch live", look: .Filled, external: true)
                    ForEach(camera.copies, id: \.label) { CopyButton(copy: $0) }
                }
            }
            ForEach(camera.facts, id: \.label) { DeviceFact(label: $0.label, value: $0.value) }
            if let bitrate = camera.bitrate { DeviceChart(chart: bitrate) }
            if let diagnosis = camera.diagnosis {
                DeviceNotice(notice: diagnosis)
                if let fix = camera.fix {
                    Button(fix.label) { onWizard(fix.scan) }.buttonStyle(.filled).padding(.horizontal, 16)
                }
                Text("This page keeps checking — it updates by itself the moment the camera answers again.")
                    .font(.bodySmall)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 4)
            }
            if camera.fix == nil {
                ReconfigureButton(live: camera.live, onWizard: onWizard)
            }
            if !camera.links.isEmpty {
                CameraLinks(camera: camera, onOpen: onOpen)
            }
            if !camera.chips.isEmpty {
                DeviceButtonRow {
                    ForEach(camera.chips, id: \.key) { chip in
                        Button { onOpen(chip.href) } label: {
                            HStack(spacing: 8) {
                                Circle().fill(deviceLevelColor(chip.level, theme)).frame(width: 8, height: 8)
                                Text(chip.label)
                                Image(.chevronRight).font(.system(size: 12))
                            }
                        }
                        .buttonStyle(.bordered)
                    }
                }
            }
        }
    }
}

private struct ReconfigureButton: View {
    let live: Bool
    let onWizard: (Bool) -> Void

    var body: some View {
        HStack {
            if live {
                Button("Reconfigure") { onWizard(false) }.buttonStyle(.bordered)
            } else {
                Button("Reconfigure") { onWizard(true) }.buttonStyle(.filled)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

private struct CameraLinks: View {
    let camera: CameraView
    let onOpen: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        if camera.live {
            VStack(alignment: .leading, spacing: 0) {
                Text("Every way to watch this camera")
                    .font(.labelLarge)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .padding(EdgeInsets(top: 12, leading: 16, bottom: 0, trailing: 16))
                EndpointRows(endpoints: camera.links)
                MoreWays(onOpen: onOpen)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Every way to watch this camera")
        } else {
            DeviceDisclosure("Links — they will play once it's back") {
                EndpointRows(endpoints: camera.links)
                MoreWays(onOpen: onOpen)
            }
        }
    }
}

private struct MoreWays: View {
    let onOpen: (String) -> Void

    var body: some View {
        NoteLink(text: "Didn't play, or the ground station is another machine?", label: "Ground station links →") { onOpen("/connect") }
    }
}

private struct CopyButton: View {
    let copy: CameraCopy
    @Environment(\.LocalChangeNotice) private var notice
    @State private var showingQr = false

    var body: some View {
        let title = copy.label.hasPrefix("Copy ") ? String(copy.label.dropFirst(5)) : copy.label
        HStack(spacing: 0) {
            Button { copyToClipboard(copy.text, notice) } label: {
                HStack(spacing: 8) {
                    Image(.contentCopy)
                    Text(copy.label)
                }
            }
            .buttonStyle(.bordered)
            if copy.qr {
                Button { showingQr = true } label: { Image(.qrCode2).frame(width: 48, height: 48).contentShape(Rectangle()) }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Show \(title) as a QR code")
            }
        }
        .sheet(isPresented: $showingQr) { QrDialog(title: title, text: copy.text) { showingQr = false } }
    }
}
