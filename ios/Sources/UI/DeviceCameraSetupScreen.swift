import SwiftUI

private let DEST_CORNER: CGFloat = 12

struct DeviceCameraSetupScreen: View {
    let props: DeviceScreenProps
    @DeviceView("cameras") private var reading

    var body: some View {
        let view = deviceView(reading, parseCameras)
        let wanted = props.query["camera"] ?? ""
        let camera = view?.cameras.first(where: { $0.name == wanted })
        if view == nil {
            FootNote(text: DEVICE_WAITING)
        } else if let camera {
            CameraSetupParts(
                camera: camera,
                opened: openedSection(props.href),
                onOpen: props.onOpen,
                onOpenFrom: props.onOpenFrom,
                onClose: props.onClose
            )
            .id(camera.name)
        } else {
            DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Neutral, text: "Pick a camera on the Cameras page to set it up."))
            Button("Cameras →") { props.onOpen("/streaming") }.buttonStyle(.text).padding(.horizontal, 8)
        }
    }
}

private struct SetupSub: View {
    let text: String
    @Environment(\.theme) private var theme

    var body: some View {
        Text(text)
            .font(.labelLarge)
            .foregroundStyle(theme.colors.onSurfaceVariant)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(EdgeInsets(top: 12, leading: 16, bottom: 4, trailing: 16))
    }
}

private struct CameraSetupParts: View {
    let camera: CameraView
    let opened: String
    let onOpen: (String) -> Void
    let onOpenFrom: (String, String) -> Void
    let onClose: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        let name = camera.name
        let setup = camera.setup
        let detectHere = cameraSectionHref(name, CameraSetupSection.Detect)
        PartDisclosure(part: setup.watch.part, section: CameraSetupSection.Watch, opened: opened) {
            WatchBody(camera: camera, onOpen: onOpen)
        }
        PartDisclosure(part: setup.radio.part, section: CameraSetupSection.Radio, opened: opened) {
            LinkedNoticeView(notice: setup.radio.notice, onOpen: onOpen)
            if setup.radio.form {
                DeviceForm(
                    fields: [
                        DeviceField(
                            name: "address",
                            label: "Address wfb_tx reads from",
                            value: setup.radio.address,
                            hint: "The default is where a stock wfb-ng install reads from.",
                            keyboard: .uri
                        ),
                    ],
                    method: .PUT,
                    path: "/api/streams/\(name)/wfb",
                    submit: "Save",
                    success: "\(name) is now going over the radio.",
                    note: "Takes effect immediately — no restart."
                )
                .id(setup.radio.address)
            }
            if setup.radio.stop {
                DeviceButtonRow {
                    ActionButton(
                        label: "Stop",
                        request: DeviceRequest(method: .DELETE, path: "/api/streams/\(name)/wfb"),
                        success: "Stopped sending \(name) over the radio.",
                        confirm: ActionConfirm(title: "Stop the radio", text: "Stop sending \(name) over the radio link?", confirm: "Stop"),
                        tonal: false
                    )
                }
            }
        }
        PartDisclosure(part: setup.servers.part, section: CameraSetupSection.Servers, opened: opened) {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(setup.servers.dests, id: \.kind) { dest in
                    PartDisclosure(part: dest.part, section: "cam-dest-\(dest.kind)", opened: opened) {
                        ServerBody(camera: camera, dest: dest, onOpen: onOpen)
                    }
                }
            }
            .clipShape(RoundedRectangle(cornerRadius: DEST_CORNER))
            .overlay(RoundedRectangle(cornerRadius: DEST_CORNER).stroke(theme.colors.outlineVariant))
            .padding(.horizontal, 16)
        }
        PartDisclosure(part: setup.shares.part, section: CameraSetupSection.Shares, opened: opened) {
            ForEach(setup.shares.links, id: \.id) { ShareRow(link: $0) }
            DeviceForm(
                fields: [
                    DeviceField(name: "label", label: "Who is it for", value: "", placeholder: "e.g. command post"),
                    DeviceField(name: "hours", label: "Stops working after", value: setup.shares.hours.first?.value ?? "24", options: setup.shares.hours),
                ],
                method: .POST,
                path: "/api/share",
                submit: "Create link",
                success: "Share link created.",
                note: "The link opens on a page that is the picture and nothing else — no settings, no addresses.",
                extra: ["camera": name]
            )
            DeviceText(text: "A link stops working when it expires or when you turn it off. It is not a lock: anyone who can already reach this device on your network can watch the camera anyway.")
        }
        PartDisclosure(part: setup.detect.part, section: CameraSetupSection.Detect, opened: opened) {
            DeviceSwitchRow(
                title: "Draw detection boxes for \(name)",
                checked: setup.detect.on,
                on: DeviceRequest(method: .PUT, path: "/api/streams/\(name)/detect"),
                off: DeviceRequest(method: .DELETE, path: "/api/streams/\(name)/detect"),
                success: "Detection turned on for \(name).",
                offSuccess: "Detection turned off for \(name)."
            )
            DeviceText(
                text: "Draw object-detection boxes over this camera. The video is untouched — boxes are sent as data and drawn by viewers, so there's no added latency. " +
                    "The model, and whether it runs on the CPU or the Hailo-8, is set on the Detection page."
            )
            if !setup.detect.model.isEmpty {
                NoteLink(text: setup.detect.model, label: "Change model →", scale: .bodyMedium) { onOpenFrom("/settings/detection", detectHere) }
            }
            if let notice = setup.detect.notice {
                LinkedNoticeView(notice: notice) { onOpenFrom($0, detectHere) }
            }
        }
        PartDisclosure(part: setup.record.part, section: CameraSetupSection.Record, opened: opened) {
            DeviceSwitchRow(
                title: "Record \(name) to disk",
                checked: setup.record.on,
                on: DeviceRequest(method: .PUT, path: "/api/streams/\(name)/record"),
                off: DeviceRequest(method: .DELETE, path: "/api/streams/\(name)/record"),
                success: "Recording turned on for \(name).",
                offSuccess: "Recording turned off for \(name) — download it under Settings → Recordings."
            )
            DeviceText(
                text: "Recorded in ten-minute segments to this device's storage, and playable as soon as a segment closes. " +
                    "The oldest go when the size limit is reached, so recording can never fill the card and take the device down with it."
            )
            if setup.record.full {
                DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Warn, text: "Recordings have reached the size limit, so the oldest are being deleted to make room for new ones."))
            }
            if !setup.record.usage.isEmpty {
                NoteLink(text: setup.record.usage, label: "Open recordings →", scale: .bodyMedium) { onOpen("/settings/recordings") }
            }
        }
        if let gimbal = setup.gimbal {
            PartDisclosure(part: gimbal.part, section: CameraSetupSection.Gimbal, opened: opened) {
                GimbalBody(name: name, owner: gimbal.owner)
            }
        }
        HStack {
            Spacer(minLength: 0)
            ActionButton(
                label: "Remove this camera",
                request: DeviceRequest(method: .DELETE, path: "/api/streams/\(name)"),
                success: "Camera \(name) removed.",
                confirm: ActionConfirm(
                    title: "Remove camera",
                    text: "Remove camera \(name)? Its stream URLs stop working, and once no other camera uses its port the port returns to the rescue network.",
                    confirm: "Remove"
                ),
                tonal: false,
                onDone: onClose
            )
        }
        .padding(16)
    }
}

private struct GimbalBody: View {
    let name: String
    let owner: Bool

    var body: some View {
        if owner {
            DeviceText(text: "Tracking from QGroundControl steers the gimbal from this camera's picture. Only one camera can be on the gimbal — naming another one moves tracking there.")
            DeviceButtonRow {
                ActionButton(
                    label: "Not this camera",
                    request: DeviceRequest(method: .DELETE, path: "/api/streams/\(name)/gimbal/owner"),
                    success: "Tracking turned off for \(name).",
                    tonal: false
                )
            }
        } else {
            DeviceText(text: "This device has a gimbal, but it is not this camera's, so QGroundControl cannot track from this picture. Say it is the gimbal camera to track from it.")
            DeviceButtonRow {
                ActionButton(
                    label: "This is the gimbal camera",
                    request: DeviceRequest(method: .PUT, path: "/api/streams/\(name)/gimbal/owner"),
                    success: "\(name) is now the gimbal camera."
                )
            }
        }
    }
}

private struct WatchBody: View {
    let camera: CameraView
    let onOpen: (String) -> Void

    var body: some View {
        let name = camera.name
        let watch = camera.setup.watch
        SetupSub(text: "From anywhere")
        if watch.cloud {
            EndpointRows(endpoints: watch.cloudLinks)
            DeviceText(text: watch.cloudCopy)
            if watch.signIn {
                LinkedNoticeView(
                    notice: LinkedNotice(
                        level: DeviceLevel.Warn,
                        text: "Add cloud credentials in Engineering → Publishing sign-in to finish authorizing the stream.",
                        link: NoticeLink(label: "Engineering", href: "/settings/engineering")
                    ),
                    onOpen: onOpen
                )
            }
            DeviceButtonRow {
                ActionButton(
                    label: "Disconnect",
                    request: DeviceRequest(method: .DELETE, path: "/api/stream/sfu/\(name)/cloud"),
                    success: "\(name) disconnected from the cloud.",
                    confirm: ActionConfirm(
                        title: "Disconnect from the cloud",
                        text: "Disconnect \(name) from the Aircast cloud? Its cloud watch page stops working. Share links keep working on this network.",
                        confirm: "Disconnect"
                    ),
                    tonal: false
                )
            }
        } else if watch.canConnect {
            DeviceText(text: "Optional: a watch page on the Aircast cloud, reachable from outside this network.")
            DeviceButtonRow {
                ActionButton(
                    label: "Connect to cloud",
                    request: DeviceRequest(method: .POST, path: "/api/stream/sfu/\(name)/cloud"),
                    success: "Connecting \(name) to the Aircast cloud…"
                )
            }
        } else {
            LinkedNoticeView(
                notice: LinkedNotice(
                    level: DeviceLevel.Neutral,
                    text: "Needs a cloud account on this device — Device → Cloud account.",
                    link: NoticeLink(label: "Device", href: "/settings/device")
                ),
                onOpen: onOpen
            )
        }
        SetupSub(text: "On this network")
        EndpointRows(endpoints: watch.links)
        NoteLink(text: "Didn't play, or the ground station is another machine?", label: "Ground station links →") { onOpen("/connect") }
    }
}

private let STOP_LABELS = ["whip": "the SFU", "livekit": "LiveKit", "cloudflare": "Cloudflare"]

private struct ServerBody: View {
    let camera: CameraView
    let dest: ServerDest
    let onOpen: (String) -> Void
    @Environment(\.LocalDeviceOrigin) private var origin

    var body: some View {
        let name = camera.name
        let path = "/api/stream/sfu/\(name)/\(dest.kind)"
        let label = STOP_LABELS[dest.kind] ?? dest.kind
        switch dest.kind {
        case "whip":
            DeviceForm(
                fields: [DeviceField(name: "url", label: "WHIP URL", value: dest.url, placeholder: "https://sfu.example.com/whip/device", keyboard: .uri)],
                method: .PUT,
                path: path,
                submit: "Save",
                success: "Publishing \(name) to the SFU."
            )
            .id(dest.url)
            NoteLink(text: "Signs in with the device's sign-in method.", label: "Publishing sign-in →") { onOpen("/settings/engineering") }
        case "livekit":
            DeviceForm(
                fields: [
                    DeviceField(name: "url", label: "Server URL", value: dest.url, placeholder: "wss://livekit.example.com", hint: "The room is this camera's name.", keyboard: .uri),
                    DeviceField(name: "livekitKey", label: "API key", value: dest.key, placeholder: "APIxxxxxxxx"),
                    DeviceField(name: "livekitSecret", label: "API secret", value: dest.secret, placeholder: "LiveKit API secret", keyboard: .password),
                ],
                method: .PUT,
                path: path,
                submit: "Save",
                success: "Publishing \(name) to LiveKit.",
                note: "One key pair is shared by every camera published to LiveKit — changing it here changes it everywhere."
            )
            .id("\(dest.url)|\(dest.key)|\(dest.secret)")
        case "cloudflare":
            CloudflareForm(name: name, dest: dest).id("\(dest.app)|\(dest.track)|\(dest.secret)")
        default:
            EmptyView()
        }
        if dest.on {
            DeviceButtonRow {
                ActionButton(
                    label: "Stop publishing",
                    request: DeviceRequest(method: .DELETE, path: path),
                    success: "\(name) stopped publishing to \(label).",
                    confirm: ActionConfirm(title: "Stop publishing", text: "Stop publishing \(name) to \(label)?", confirm: "Stop"),
                    tonal: false
                )
            }
        }
        if dest.kind == "cloudflare" && dest.on {
            if !dest.watch.isEmpty {
                DeviceButtonRow {
                    LinkButton(href: resolveDeviceUrl(origin, dest.watch), text: "▶ Watch via Cloudflare", look: .Filled, external: true)
                }
            }
            DeviceText(
                text: "This player pulls the stream back from Cloudflare — proof of what your viewers get. There's no public watch URL: a viewer app pulls track \(dest.track) from app \(dest.app) " +
                    "through Cloudflare's Sessions API, authorized with the app secret."
            )
        }
    }
}

private struct CloudflareForm: View {
    let name: String
    @Environment(\.theme) private var theme
    @Environment(\.LocalChangeNotice) private var notice
    @State private var action = DeviceActionState()
    @State private var app: String
    @State private var secret: String
    @State private var track: String

    init(name: String, dest: ServerDest) {
        self.name = name
        _app = State(initialValue: dest.app)
        _secret = State(initialValue: dest.secret)
        _track = State(initialValue: dest.track)
    }

    private func save() {
        let request = DeviceRequest(
            method: .PUT,
            path: "/api/stream/sfu/\(name)/cloudflare",
            body: deviceBody(["url": cloudflareTarget(app, track), "secret": secret])
        )
        let success = "Publishing \(name) to Cloudflare."
        let notice = notice, action = action
        Task { @MainActor in _ = await action.run(request, success: success, notice: notice) }
    }

    var body: some View {
        let urlRefusal = action.refusal.flatMap { $0.field == "url" ? $0 : nil }
        let secretRefusal = action.refusal.flatMap { $0.field == "secret" ? $0 : nil }
        VStack(alignment: .leading, spacing: 12) {
            PlanFlowRow(spacing: 8, lineSpacing: 8) {
                Text("In the Cloudflare dashboard, open Realtime → SFU and create an app — it gives you the App ID and App secret.")
                    .font(.bodySmall)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                LinkButton(href: "https://dash.cloudflare.com/", text: "Cloudflare dashboard", look: .Outlined, external: true)
            }
            DeviceTextField(
                label: "App ID",
                value: app,
                onValueChange: { app = $0 },
                placeholder: "Realtime → SFU → your app",
                isError: urlRefusal != nil,
                supporting: urlRefusal?.message ?? "",
                keyboard: .uri
            )
            SecretField(
                label: "App secret",
                value: secret,
                onValueChange: { secret = $0 },
                placeholder: "Realtime app secret",
                isError: secretRefusal != nil,
                supportingText: secretRefusal?.message ?? ""
            )
            DeviceTextField(
                label: "Track name",
                value: track,
                onValueChange: { track = $0 },
                supporting: "Not from Cloudflare — a name you pick. Viewers subscribe to this stream by this name. The camera name works fine.",
                keyboard: .uri
            )
            if urlRefusal == nil && secretRefusal == nil {
                DeviceRefusalText(refusal: action.refusal, field: action.refusal?.field ?? "").padding(.horizontal, -DEVICE_LIST_PADDING)
            }
            Button("Save", action: save).buttonStyle(.bordered).disabled(action.busy)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

private struct ShareRow: View {
    let link: CameraShareLink
    @Environment(\.theme) private var theme

    var body: some View {
        if link.dead.isEmpty {
            CopyRow(label: link.label, text: link.url, hint: link.expires, qr: true)
            DeviceButtonRow {
                ActionButton(
                    label: "Turn off",
                    request: DeviceRequest(method: .DELETE, path: "/api/share/\(link.id)"),
                    success: "Link turned off.",
                    confirm: ActionConfirm(title: "Turn off this link", text: "Turn off this link? Whoever has it stops being able to open the picture.", confirm: "Turn off"),
                    tonal: false
                )
            }
        } else {
            VStack(alignment: .leading, spacing: 2) {
                Text(link.label).font(.bodyLarge)
                Text(link.dead).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .opacity(0.7)
        }
    }
}
