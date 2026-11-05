import SwiftUI
#if os(iOS)
import UIKit
#endif
import UniformTypeIdentifiers

private let BOX_CORNER: CGFloat = 12
private let WIZARD_ERROR = "camera-wizard-refusal"

private struct CameraSignIn: Equatable {
    let shown: Bool
    let asked: Bool
}

private struct PortInput: Equatable {
    let open: Bool
    let text: String
}

private struct FormInputs: Equatable {
    let rtsp: String
    let iface: String
    let ip: String
    let current: String
    let scans: Int
    let scanning: Bool
}

private let NO_SIGN_IN = CameraSignIn(shown: false, asked: false)
private let NO_WIZARD = parseDeviceWizard(nil)

private func errorOf(_ json: JSON?, _ fallback: String) -> String { (json?["error"].string ?? "").ifEmpty(fallback) }

func stagedVideoName(_ name: String) -> String {
    name.replacingOccurrences(of: "/", with: "_").ifEmpty("video.mp4")
}

private func stagedVideo(_ chosen: URL) -> URL? {
    let scoped = chosen.startAccessingSecurityScopedResource()
    defer { if scoped { chosen.stopAccessingSecurityScopedResource() } }
    let files = FileManager.default
    let folder = files.temporaryDirectory.appendingPathComponent("device-uploads", isDirectory: true)
    guard (try? files.createDirectory(at: folder, withIntermediateDirectories: true)) != nil else { return nil }
    let staged = folder.appendingPathComponent(stagedVideoName(chosen.lastPathComponent))
    try? files.removeItem(at: staged)
    return (try? files.copyItem(at: chosen, to: staged)).map { staged }
}

private struct WizardBox: ViewModifier {
    let border: Color
    var fill: Color = .clear

    func body(content: Content) -> some View {
        content
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(fill, in: RoundedRectangle(cornerRadius: BOX_CORNER))
            .overlay(RoundedRectangle(cornerRadius: BOX_CORNER).stroke(border))
    }
}

struct CameraWizard: View {
    let view: CameraWizardView
    let scan: Bool
    let onCancel: (() -> Void)?
    let onStep: (Int) -> Void
    let onDone: (String) -> Void
    @Environment(\.theme) private var theme
    @Environment(\.LocalChangeNotice) private var notice
    @Environment(\.LocalDeviceScroll) private var scroll
    @Environment(\.LocalNativeList) private var nativeList
    @QgcPath(DEVICE_WIZARD) private var shown
    @FocusState private var userFocused: Bool
    @State private var step = 1
    @State private var rtsp: String
    @State private var name: String
    @State private var addressOpen: Bool
    @State private var namingOpen = false
    @State private var techOpen = false
    @State private var portOpen = false
    @State private var fileOpen = false
    @State private var refusal: WizardRefusal?
    @State private var dupAck = ""
    @State private var inputs: [String: PortInput] = [:]
    @State private var picked: WizardCamera?
    @State private var editRtsp = false
    @State private var base = ""
    @State private var signIn = NO_SIGN_IN
    @State private var user = ""
    @State private var pass = ""
    @State private var iface: String
    @State private var ifaceTouched = false
    @State private var ip = ""
    @State private var transport: String
    @State private var form = NO_FORM
    @State private var session: Int64 = -1
    @State private var seen = NO_WIZARD
    @State private var file: URL?
    @State private var uploading = false
    @State private var pickingFile = false

    init(
        view: CameraWizardView,
        scan: Bool = false,
        onCancel: (() -> Void)? = nil,
        onStep: @escaping (Int) -> Void = { _ in },
        onDone: @escaping (String) -> Void
    ) {
        self.view = view
        self.scan = scan
        self.onCancel = onCancel
        self.onStep = onStep
        self.onDone = onDone
        _rtsp = State(initialValue: view.rtsp)
        _name = State(initialValue: view.suggestedName)
        _addressOpen = State(initialValue: view.addressOpen)
        _iface = State(initialValue: view.iface)
        _transport = State(initialValue: view.transport)
    }

    private var currentWizard: DeviceWizard {
        let parsed = parseDeviceWizard(shown)
        return parsed.session == session ? parsed : NO_WIZARD
    }

    private var locked: Bool { picked?.pathFound == true && !editRtsp }

    private var cameraName: String { view.isNew ? name.trimmed.lowercased() : view.name }

    private var currentAddress: String { view.interfaces.first(where: { $0.value == iface })?.address ?? "" }

    private func applyField(_ wizard: DeviceWizard) -> String { wizard.apply.state == "failed" ? wizard.apply.field : "" }

    @MainActor
    private func pick(_ cam: WizardCamera) async -> String {
        picked = cam
        editRtsp = false
        if !cam.pathFound { addressOpen = true }
        base = cam.url
        signIn = CameraSignIn(shown: cam.needsAuth, asked: false)
        if !cam.needsAuth {
            user = ""
            pass = ""
        }
        let url = await signedInUrl(cam.url, user, pass)
        if !cam.url.isEmpty { rtsp = url }
        return url
    }

    private func askSignIn(_ url: String, _ reason: String) {
        base = url
        signIn = CameraSignIn(shown: true, asked: true)
        step = 1
        refusal = WizardRefusal(text: reason, warn: false, field: nil)
    }

    private func applySetup(_ url: String, _ camera: String, _ port: String, _ portIp: String, _ how: String) {
        step = 3
        Task { await applyCamera(url, camera, port, portIp, how) }
    }

    @MainActor
    private func advance(_ typed: String, _ typedName: String) async {
        guard let out = await wizardNext(typed, typedName, dupAck) else { return }
        rtsp = out.rtsp
        if let given = out.name { name = given }
        dupAck = out.dupAck
        if let refused = out.refusal {
            refusal = refused
            if refused.field == "rtsp" {
                editRtsp = true
                addressOpen = true
            }
            if refused.field == "name" { namingOpen = true }
        } else if out.kind == "apply" {
            refusal = nil
            iface = ""
            ip = ""
            applySetup(out.rtsp, out.camera, "", "", transport)
        } else {
            refusal = nil
            if !out.iface.isEmpty && !ifaceTouched { iface = out.iface }
            if ip.isEmpty { ip = out.ip }
            step = 2
        }
    }

    @MainActor
    private func follow(_ now: DeviceWizard) async {
        let before = seen
        seen = now
        if now.scans > before.scans {
            picked = nil
            base = ""
            signIn = NO_SIGN_IN
            inputs = [:]
            if let adopt = now.adopt { _ = await pick(adopt) }
        }
        if now.template.n > before.template.n {
            if let found = now.template.camera {
                let url = await pick(found)
                let camera = now.template.name.ifEmpty(name)
                name = camera
                await advance(url, camera)
            } else {
                refusal = WizardRefusal(text: now.template.refusal, warn: false, field: nil)
            }
        }
        if now.notice.n > before.notice.n { notice.show(now.notice.text, nil) }
        if now.apply.n > before.apply.n {
            switch now.apply.state {
            case "live": onDone(now.apply.camera)
            case "signIn": askSignIn(now.apply.url, now.apply.text)
            default: break
            }
        }
    }

    @ViewBuilder
    private func leadButton(_ title: String, leads: Bool, quiet: Bool = false, action: @escaping () -> Void) -> some View {
        if leads {
            Button(title, action: action).buttonStyle(.filled)
        } else if quiet {
            Button(title, action: action).buttonStyle(.text)
        } else {
            Button(title, action: action).buttonStyle(.bordered)
        }
    }

    private func restart(then next: @escaping () async -> Void = {}) {
        Task {
            await restartWizard()
            await next()
        }
        rtsp = ""
        ip = ""
        transport = view.transports.first?.value ?? view.transport
        refusal = nil
        picked = nil
        base = ""
        signIn = NO_SIGN_IN
        editRtsp = false
        step = 1
    }

    private func credentials(_ nextUser: String, _ nextPass: String) {
        user = nextUser
        pass = nextPass
        let from = base
        guard !from.isEmpty else { return }
        Task { @MainActor in
            let url = await signedInUrl(from, nextUser, nextPass)
            if user == nextUser && pass == nextPass { rtsp = url }
        }
    }

    private func input(_ port: WizardPort) -> PortInput { inputs[port.iface] ?? PortInput(open: port.open, text: port.suggest) }

    private func upload() {
        let named = cameraName
        guard !named.isEmpty else { return notice.show("Name the camera first.", nil) }
        guard let chosen = file else { return notice.show("Choose a video file.", nil) }
        uploading = true
        let notice = notice, onDone = onDone
        Task { @MainActor in
            let sent = await deviceUpload(.PUT, "/api/streams/\(uriComponent(named))/file", [DevicePart(name: "file", file: chosen)])
            uploading = false
            if sent.refusal == nil {
                notice.show("Video source added.", nil)
                onDone(named)
            } else if sent.refusal == DEVICE_NO_ANSWER {
                notice.show("Upload failed — check the file size and try again.", nil)
            } else {
                notice.show(errorOf(sent.result, "Upload failed."), nil)
            }
        }
    }

    private func continueFrom(_ typed: String, _ typedName: String) {
        Task { @MainActor in await advance(typed, typedName) }
    }

    var body: some View {
        let wizard = currentWizard
        VStack(alignment: .leading, spacing: 12) {
            if !(repairing(view) && step == 1) {
                StepNames(step: step, skipsPort: step == 1 && form.skipsPort)
            }
            if step == 1 {
                findCamera(wizard)
                cameraAddress(wizard)
            }
            if step == 2 { pickPort() }
            if step == 3 { applying(wizard) }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .rowPadding(horizontal: 16, vertical: 8)
        .padding(.vertical, nativeList ? Space.s2 : 0)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(view.isNew ? "New camera" : "Set up \(view.name)")
        // The wizard is one form row, so its session hooks live on it; a list re-runs a row's task each time the row
        // scrolls back into view, so the session opens only once.
        .task {
            guard session < 0 else { return }
            session = await openWizard(view, scan)
        }
        .task(id: FormInputs(rtsp: rtsp, iface: iface, ip: ip, current: currentAddress, scans: wizard.scans, scanning: wizard.scanning)) {
            form = await wizardForm(rtsp, iface, ip, currentAddress)
        }
        .onChange(of: wizard, initial: true) { _, now in Task { @MainActor in await follow(now) } }
        .onChange(of: step, initial: true) { _, now in onStep(now) }
        .onChange(of: refusal) { _, now in
            guard let now else { return }
            withAnimation { scroll?.scrollTo(WIZARD_ERROR, anchor: .center) }
            announceForAccessibility(now.text)
        }
        .onChange(of: signIn) { _, now in
            guard now.asked else { return }
            Task { @MainActor in
                await Task.yield()
                userFocused = true
            }
        }
        .fileImporter(isPresented: $pickingFile, allowedContentTypes: [UTType.movie]) { result in
            guard case .success(let chosen) = result else { return }
            Task { @MainActor in file = await offMain { stagedVideo(chosen) } ?? file }
        }
    }

    @ViewBuilder
    private func findCamera(_ wizard: DeviceWizard) -> some View {
        if let pi = view.piCamera {
            HStack(spacing: 12) {
                Text(pi.note).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                Button(pi.action) { continueFrom(PI_CAMERA, name) }.buttonStyle(.filled)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: BOX_CORNER))
        }
        if !wizard.templates.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                ForEach(wizard.templates, id: \.id) { make in
                    Button {
                        refusal = nil
                        Task { await setUpCameraMake(make.id) }
                    } label: {
                        HStack(spacing: 12) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(make.label).font(.bodyLarge).foregroundStyle(theme.colors.onSurface)
                                Text(make.sub).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            Text(make.action).font(.labelLarge).foregroundStyle(theme.colors.primary)
                        }
                        .modifier(WizardBox(border: theme.colors.outlineVariant))
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .disabled(wizard.templateBusy)
                    .opacity(wizard.templateBusy ? 0.6 : 1)
                }
                if !wizard.templateNote.isEmpty { DeviceHint(text: wizard.templateNote) }
            }
        }
        if !view.lookingFor.isEmpty {
            Text(view.lookingFor).font(.titleSmall).foregroundStyle(theme.aircast.warning)
        }
        if wizard.checklist.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                if !repairing(view) { DeviceHint(text: "Plug the camera in and look for it.") }
                LeadButton(leads: scanLeads(rtsp, picked != nil), label: wizard.scanning ? "Looking…" : "Scan for cameras", enabled: !wizard.scanning) {
                    Task { await scanForCameras() }
                }
            }
        }
        if !wizard.note.isEmpty { DeviceHint(text: wizard.note) }
        if !wizard.checklist.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                Text(wizard.checklist).font(.titleSmall)
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(Array(view.checks.enumerated()), id: \.offset) { _, check in
                        Text("☐ \(check)").font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                }
                PlanFlowRow(spacing: 12, lineSpacing: 8) {
                    Button(wizard.scanning ? "Looking…" : "Scan again") { Task { await scanForCameras() } }
                        .buttonStyle(.filled)
                        .disabled(wizard.scanning)
                    if wizard.again > 0 {
                        Text("Checking again in \(wizard.again) s…").font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                }
            }
            .modifier(WizardBox(border: theme.colors.outlineVariant))
        }
        if !wizard.cams.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                Text(wizard.camHeading).font(.labelLarge).foregroundStyle(theme.colors.onSurfaceVariant)
                ForEach(wizard.cams, id: \.self) { cam in
                    FoundCamera(cam: cam, selected: picked == cam) { Task { @MainActor in _ = await pick(cam) } }
                }
                if !wizard.camsHint.isEmpty { DeviceHint(text: wizard.camsHint) }
            }
        }
        if !wizard.details.isEmpty && (!repairing(view) || !wizard.todo.isEmpty) {
            WizardFold(title: wizard.details, open: techOpen, onToggle: { techOpen = $0 }) {
                ScanDetails(
                    wizard: wizard,
                    coverage: view.coverage,
                    input: { input($0) },
                    onInput: { port, change in inputs[port.iface] = change },
                    onFix: { port in
                        let address = input(port).text.trimmed
                        Task { await fixWizardPort(port.iface, address) }
                    }
                )
            }
        }
    }

    @ViewBuilder
    private func cameraAddress(_ wizard: DeviceWizard) -> some View {
        let refusedField = applyField(wizard)
        if signIn.shown {
            VStack(alignment: .leading, spacing: 12) {
                DeviceHint(text: "This camera asked for a sign-in. Enter the username and password set in the camera's own app.")
                WizardField(label: "Camera username", value: user, onValueChange: { credentials($0, pass) }, focus: $userFocused)
                SecretField(label: "Camera password", value: pass, onValueChange: { credentials(user, $0) })
            }
            .modifier(WizardBox(border: theme.colors.outlineVariant))
        }
        if locked {
            HStack(spacing: 8) {
                Text("Stream").font(.labelLarge).foregroundStyle(theme.colors.onSurfaceVariant)
                Text(base).font(deviceMono(.bodyMedium)).lineLimit(1).truncationMode(.tail).frame(maxWidth: .infinity, alignment: .leading)
                Button("Change") {
                    editRtsp = true
                    addressOpen = true
                }
                .buttonStyle(.text)
            }
        } else {
            WizardFold(title: "I know my camera's address", open: addressOpen, onToggle: { addressOpen = $0 }) {
                WizardField(
                    label: "Camera RTSP URL",
                    value: rtsp,
                    onValueChange: { rtsp = $0 },
                    placeholder: view.rtspHint,
                    keyboard: .uri,
                    isError: refusal?.field == "rtsp" || refusedField == "rtsp",
                    supporting: picked == nil ? "Find it in the camera's app or manual — for example rtsp://192.168.8.10:554/stream" : ""
                )
            }
        }
        if view.isNew {
            let shownName = name.trimmed.lowercased()
            WizardFold(title: "Named \(shownName.ifEmpty("…")) — change", open: namingOpen, onToggle: { namingOpen = $0 }) {
                WizardField(
                    label: "Camera name",
                    value: name,
                    onValueChange: { name = $0 },
                    placeholder: "cam",
                    isError: refusal?.field == "name" || refusedField == "name",
                    supporting: "Used in the stream URLs (rtsp://…/\(view.suggestedName)) — lowercase letters, numbers, dashes."
                )
            }
        }
        if let refused = refusal {
            let tint = refused.warn ? theme.aircast.warning : theme.colors.error
            HStack(alignment: .top, spacing: 8) {
                Image(.warning).font(.system(size: 16)).foregroundStyle(tint)
                Text(refused.text).font(.bodyMedium).foregroundStyle(tint).frame(maxWidth: .infinity, alignment: .leading)
            }
            .id(WIZARD_ERROR)
            .accessibilityElement(children: .combine)
        }
        PlanFlowRow(spacing: 8, lineSpacing: 8) {
            if nextOffered(view, rtsp, picked: picked != nil) {
                LeadButton(leads: continueLeads(rtsp), label: form.next) { continueFrom(rtsp, name) }
            }
            if let onCancel {
                Button("Cancel", action: onCancel).buttonStyle(.text)
            }
        }
        if view.isNew {
            WizardFold(title: "No camera? Use a video file", open: fileOpen, onToggle: { fileOpen = $0 }) {
                DeviceHint(text: "Upload a video and the device loops it as a stream — a stand-in camera for trying out detection and the overlays.")
                HStack(spacing: 8) {
                    Button("Choose a video file") { pickingFile = true }.buttonStyle(.bordered)
                    Text(file?.lastPathComponent ?? "H.264 .mp4 works best")
                        .font(.bodyMedium)
                        .foregroundStyle(theme.colors.onSurfaceVariant)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                Button(uploading ? "Uploading…" : "Upload & start") { upload() }
                    .buttonStyle(.bordered)
                    .disabled(uploading)
            }
        }
    }

    @ViewBuilder
    private func pickPort() -> some View {
        let refusedField = applyField(currentWizard)
        DeviceSelect(
            label: "Which port is the camera plugged into?",
            value: iface,
            options: [DeviceOption(value: "", label: "Select a port")] + view.interfaces.map { DeviceOption(value: $0.value, label: $0.label) },
            onPick: { port in
                iface = port
                ifaceTouched = true
            }
        )
        DeviceHint(text: "Usually the Ethernet port your camera's cable connects to. Each port shows its own address — the camera has to be on that same subnet to be reachable.")
        WizardFold(
            title: "Port address & transport",
            open: portOpen,
            onToggle: { portOpen = $0 },
            summary: "This device's address must be on the same subnet as the camera's URL above — change it only if yours differs."
        ) {
            WizardField(
                label: "This device's IP on the camera subnet",
                value: ip,
                onValueChange: { ip = $0 },
                placeholder: "192.168.8.2/24",
                keyboard: .uri,
                isError: refusedField == "address",
                supporting: "Suggested from the camera's address — edit if your subnet differs."
            )
            DeviceSelect(label: "RTSP transport", value: transport, options: view.transports, onPick: { transport = $0 })
            DeviceHint(text: "Leave on Automatic; switch to TCP if the video stays black or choppy.")
        }
        HStack(spacing: 8) {
            Button("Back") { step = 1 }.buttonStyle(.bordered)
            Spacer(minLength: 0)
            Button("Set up & verify") { applySetup(rtsp, cameraName, iface, ip, transport) }.buttonStyle(.filled)
        }
        if !form.applyNote.isEmpty { DeviceHint(text: form.applyNote) }
    }

    @ViewBuilder
    private func applying(_ wizard: DeviceWizard) -> some View {
        let failed = wizard.apply.state == "failed"
        HStack(spacing: 12) {
            if failed {
                Image(.error).font(.system(size: 20)).foregroundStyle(theme.colors.error)
            } else {
                ProgressView().accessibilityLabel("Working")
            }
            Text(wizard.apply.text.ifEmpty("Applying…"))
                .font(.bodyLarge)
                .foregroundStyle(failed ? theme.colors.error : theme.colors.onSurface)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .accessibilityElement(children: .combine)
        HStack(spacing: 8) {
            if failed {
                leadButton("Scan for cameras", leads: wizard.apply.lead == "scan", quiet: true) { restart { await scanForCameras() } }
            } else {
                Button("Start over") { restart() }.buttonStyle(.text)
            }
            Spacer(minLength: 0)
            if failed {
                leadButton("Change settings", leads: wizard.apply.lead == "settings") { step = form.skipsPort ? 1 : 2 }
                leadButton("Try again", leads: wizard.apply.lead == "retry") { applySetup(rtsp, cameraName, iface, ip, transport) }
            }
        }
    }
}

// A fold inside the wizard: a native DisclosureGroup in a NativeForm, the hand-drawn Fold elsewhere.
private struct WizardFold<Content: View>: View {
    let title: String
    let open: Bool
    let onToggle: (Bool) -> Void
    var summary: String = ""
    @ViewBuilder let content: () -> Content
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            DisclosureGroup(isExpanded: Binding(get: { open }, set: onToggle)) {
                VStack(alignment: .leading, spacing: 12) { content() }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.top, Space.s2)
            } label: {
                VStack(alignment: .leading, spacing: 2) {
                    Text(title).foregroundStyle(Color.primary)
                    if !summary.isEmpty {
                        Text(summary).font(.footnote).foregroundStyle(Color.secondary)
                    }
                }
            }
        } else {
            Fold(title: title, open: open, onToggle: onToggle, summary: summary, content: content)
        }
    }
}

private struct FoundCamera: View {
    let cam: WizardCamera
    let selected: Bool
    let onPick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onPick) {
            HStack(spacing: 12) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(cam.host).font(deviceMono(.bodyLarge)).foregroundStyle(theme.colors.onSurface)
                    Text(cam.note).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                Text(selected ? "Selected" : cam.action).font(.labelLarge).foregroundStyle(theme.colors.primary)
            }
            .modifier(WizardBox(border: selected ? theme.colors.primary : theme.colors.outlineVariant, fill: selected ? theme.colors.primary.opacity(0.08) : Color.clear))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

private struct LeadButton: View {
    let leads: Bool
    let label: String
    var enabled: Bool = true
    let action: () -> Void

    var body: some View {
        if leads {
            Button(label, action: action).buttonStyle(.filled).disabled(!enabled)
        } else {
            Button(label, action: action).buttonStyle(.bordered).disabled(!enabled)
        }
    }
}

private struct WizardField: View {
    let label: String
    let value: String
    let onValueChange: (String) -> Void
    var placeholder: String = ""
    var keyboard: DeviceKeyboard = .text
    var isError: Bool = false
    var supporting: String = ""
    var focus: FocusState<Bool>.Binding? = nil
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            // As FieldRow: the label left, the value typed on the right, the hint or error underneath.
            VStack(alignment: .leading, spacing: Space.s1) {
                LabeledContent {
                    input.multilineTextAlignment(.trailing)
                } label: {
                    Text(label).foregroundStyle(isError ? theme.colors.error : Color.primary)
                }
                if !supporting.isEmpty {
                    Text(supporting).font(.footnote).foregroundStyle(isError ? theme.colors.error : Color.secondary)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        } else {
            outlined
        }
    }

    private var outlined: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label).font(.bodySmall).foregroundStyle(isError ? theme.colors.error : theme.colors.onSurfaceVariant)
            input
                .padding(12)
                .overlay(RoundedRectangle(cornerRadius: Corner.extraSmall).stroke(isError ? theme.colors.error : theme.colors.outline))
            if !supporting.isEmpty {
                Text(supporting).font(.bodySmall).foregroundStyle(isError ? theme.colors.error : theme.colors.onSurfaceVariant)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    @ViewBuilder
    private var input: some View {
        let field = TextField(placeholder, text: Binding(get: { value }, set: onValueChange))
            .font(.bodyLarge)
            .keyboardType(keyboard == .uri ? UIKeyboardType.URL : UIKeyboardType.default)
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
            .submitLabel(.done)
        if let focus {
            field.focused(focus)
        } else {
            field
        }
    }
}

private struct StepNames: View {
    let step: Int
    let skipsPort: Bool
    @Environment(\.theme) private var theme

    var body: some View {
        PlanFlowRow(spacing: 8, lineSpacing: 8) {
            ForEach(Array(STEP_NAMES.enumerated()), id: \.offset) { index, label in
                let skipped = index == 1 && skipsPort
                HStack(spacing: 8) {
                    Capsule()
                        .fill(index + 1 <= step ? theme.colors.primary : theme.colors.outlineVariant)
                        .frame(width: 24, height: 6)
                        .opacity(skipped ? 0.4 : 1)
                    Text(label)
                        .strikethrough(skipped)
                        .font(.labelMedium)
                        .foregroundStyle(index + 1 == step ? theme.colors.onSurface : theme.colors.onSurfaceVariant)
                }
                .accessibilityElement(children: .combine)
                .accessibilityAddTraits(index + 1 == step ? .isSelected : [])
            }
        }
    }
}

private struct PortRow: View {
    let port: WizardPort
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(port.label).font(.bodyLarge)
            Text(port.detail).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
        }
        .modifier(WizardBox(border: theme.colors.outlineVariant))
    }
}

private struct RestPorts: View {
    let ports: [WizardPort]

    var body: some View {
        ForEach(ports, id: \.iface) { PortRow(port: $0) }
    }
}

private struct ScanDetails: View {
    let wizard: DeviceWizard
    let coverage: String
    let input: (WizardPort) -> PortInput
    let onInput: (WizardPort, PortInput) -> Void
    let onFix: (WizardPort) -> Void
    @Environment(\.theme) private var theme
    @State private var restOpen = false
    @State private var checkedOpen = false

    var body: some View {
        if !wizard.todo.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                Text(wizard.todoHeading).font(.labelLarge).foregroundStyle(theme.colors.onSurfaceVariant)
                ForEach(wizard.todo, id: \.iface) { port in
                    PortFix(port: port, input: input(port), onInput: { onInput(port, $0) }, onFix: { onFix(port) })
                }
            }
        }
        if !wizard.rest.isEmpty && !wizard.todo.isEmpty {
            WizardFold(title: wizard.restTitle, open: restOpen, onToggle: { restOpen = $0 }) { RestPorts(ports: wizard.rest) }
        }
        if !wizard.rest.isEmpty && wizard.todo.isEmpty && !wizard.cams.isEmpty {
            RestPorts(ports: wizard.rest)
            DeviceHint(text: coverage)
        }
        if !wizard.rest.isEmpty && wizard.todo.isEmpty && wizard.cams.isEmpty {
            Text("Ports checked").font(.labelLarge).foregroundStyle(theme.colors.onSurfaceVariant)
            RestPorts(ports: wizard.rest)
        }
        if wizard.cams.isEmpty {
            WizardFold(title: "What was checked?", open: checkedOpen, onToggle: { checkedOpen = $0 }) { DeviceHint(text: coverage) }
        }
    }
}

private struct PortFix: View {
    let port: WizardPort
    let input: PortInput
    let onInput: (PortInput) -> Void
    let onFix: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(port.label).font(.bodyLarge)
            Text(port.detail).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            if input.open {
                WizardField(
                    label: "This port's address",
                    value: input.text,
                    onValueChange: { onInput(PortInput(open: input.open, text: $0)) },
                    placeholder: "192.168.144.2/24",
                    keyboard: .uri
                )
                PlanFlowRow(spacing: 8, lineSpacing: 8) {
                    ForEach(port.options, id: \.cidr) { option in
                        DeviceChip(label: option.text, selected: false) { onInput(PortInput(open: input.open, text: option.cidr)) }
                    }
                }
                DeviceHint(text: port.hint)
                LeadButton(leads: !port.suggest.isEmpty, label: port.go, enabled: !port.busy && !input.text.trimmed.isEmpty, action: onFix)
                if !port.status.isEmpty { DeviceHint(text: port.status) }
            } else {
                Button("Change this port's address") { onInput(PortInput(open: true, text: input.text)) }.buttonStyle(.bordered)
            }
        }
        .modifier(WizardBox(border: theme.colors.primary))
    }
}
