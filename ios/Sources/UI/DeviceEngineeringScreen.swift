import SwiftUI

struct DeviceEngineeringScreen: View {
    let props: DeviceScreenProps
    @DeviceView("engineering") private var reading
    @Environment(\.LocalDeviceOrigin) private var origin

    var body: some View {
        NativeForm {
            if let view = deviceView(reading, parseEngineering) {
                BackendSection(backend: view.backend)
                PublishSection(publish: view.publish, onOpen: props.onOpen)
                Section {
                    LinkButton(href: resolveDeviceUrl(origin, "/api/logs"), text: "Download logs", look: .Text, icon: .download)
                } header: {
                    SectionHeader(text: "Device logs")
                } footer: {
                    Text("Everything every service on this device has logged, gzipped.")
                }
                DiagnosticsSection(diagnostics: view.diagnostics)
            } else {
                FootNote(text: DEVICE_WAITING)
            }
        }
    }
}

private struct BackendSection: View {
    let backend: BackendChoice

    var body: some View {
        DeviceSection(title: "What carries video", badge: backend.status) {
            DeviceStatusLine(text: "aircastd's own engine, or mediamtx as a separate service — switching restarts the device.")
            DeviceText(text: backend.text)
            DeviceButtonRow {
                ActionButton(
                    label: backend.label,
                    request: DeviceRequest(method: .PUT, path: "/api/stream/media-backend", body: deviceBody(["backend": backend.switchTo])),
                    success: backend.success,
                    confirm: ActionConfirm(title: "Restart the device?", text: backend.confirm, confirm: "Switch")
                )
            }
            DeviceGroup("Technical details") {
                DeviceText(
                    text: "Both backends bind the same ports — 8554, 8888, 8889, 1935, 8890 — so only one can run, and the choice is read once at startup. Switching writes the setting and restarts aircastd into it; " +
                        "if the restart fails the setting still stands and the next boot lands on it. Camera paths carry across in both directions. Only the engine can send a camera to a wfb-ng packet radio."
                )
            }
        }
    }
}

private struct PublishSection: View {
    let publish: PublishSignIn
    let onOpen: (String) -> Void

    var body: some View {
        DeviceSection(title: "Publishing sign-in", badge: publish.status) {
            DeviceText(
                text: "Devices on the Aircast cloud sign in automatically once connected to a cloud account. Custom servers are set up under Advanced; publishing to a specific server — LiveKit, Cloudflare — " +
                    "is configured per camera on the Cameras page."
            )
            if publish.awaitingCloud {
                SetupRow(title: "Connect the cloud account", onClick: { onOpen("/settings/device") }, subtitle: "Nothing to fill in here — the sign-in arrives with the cloud account.")
            }
            SetupRow(title: "Choose where cameras publish", onClick: { onOpen("/streaming") })
            DeviceGroup("Advanced · sign-in credentials", summary: "How this device signs in to video servers.") {
                SignInEditor(publish: publish).id(signInKey(publish))
            }
        }
    }
}

private struct SignInInput: View {
    let field: SignInField
    let value: String
    let refusal: DeviceRefusal?
    let onValueChange: (String) -> Void

    var body: some View {
        let refused = refusal?.field == field.name
        let supporting = refused ? refusal?.message ?? "" : ""
        if field.secret {
            SecretField(label: field.label, value: value, onValueChange: onValueChange, placeholder: field.placeholder, isError: refused, supportingText: supporting)
        } else {
            DeviceTextField(label: field.label, value: value, onValueChange: onValueChange, placeholder: field.placeholder, isError: refused, supporting: supporting, keyboard: .uri)
        }
    }
}

private struct SignInEditor: View {
    let publish: PublishSignIn
    @Environment(\.theme) private var theme
    @Environment(\.LocalChangeNotice) private var notice
    @State private var action = DeviceActionState()
    @State private var mode: String
    @State private var edits: [String: String] = [:]
    @State private var asking = false

    init(publish: PublishSignIn) {
        self.publish = publish
        _mode = State(initialValue: publish.mode)
    }

    private func save() {
        let request = DeviceRequest(method: .PUT, path: "/api/stream/sfu/oauth", body: deviceBody(signInBody(publish, mode, edits)))
        let notice = notice, action = action
        Task { @MainActor in _ = await action.run(request, success: "Publishing sign-in saved.", notice: notice) }
    }

    var body: some View {
        let picked = modeNamed(publish, mode)
        let dirty = mode != publish.mode
        let confirm = dirty ? picked?.confirm ?? "" : ""
        let fields = signInFields(publish, mode)
        // Rows inside the Advanced group: one checkmark row per sign-in method, then its fields, the verdict and Save.
        ForEach(publish.modes, id: \.value) { option in
            Button { mode = option.value } label: {
                HStack(spacing: Space.s3) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(option.title).foregroundStyle(Color.primary)
                        Text(option.desc).font(.subheadline).foregroundStyle(Color.secondary)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    if option.value == mode {
                        Image(systemName: "checkmark").fontWeight(.semibold).foregroundStyle(theme.colors.primary)
                    }
                }
                .contentShape(Rectangle())
            }
            .listRowButton()
            .accessibilityAddTraits(option.value == mode ? .isSelected : [])
        }
        if mode == "aircast" && !publish.environment.isEmpty {
            LabeledContent("Environment") {
                Text(publish.environment).font(deviceMono(.bodyMedium)).foregroundStyle(Color.secondary)
            }
        }
        ForEach(fields, id: \.name) { field in
            SignInInput(field: field, value: edits[field.name] ?? field.value, refusal: action.refusal) { edits[field.name] = $0 }
        }
        if dirty {
            Text(picked?.unsaved ?? "").font(.footnote).foregroundStyle(Color.secondary)
        } else if publish.verdict.level == DeviceLevel.Blocked {
            DeviceNotice(notice: publish.verdict)
        } else {
            Text(publish.verdict.text).font(.subheadline).foregroundStyle(Color.secondary)
        }
        if let refusal = action.refusal, !fields.contains(where: { $0.name == refusal.field }) {
            DeviceRefusalText(refusal: refusal, field: refusal.field)
        }
        HStack(spacing: Space.s2) {
            Text("Stored on the device only.").font(.footnote).foregroundStyle(Color.secondary).frame(maxWidth: .infinity, alignment: .leading)
            Button(picked?.submit ?? "Save") { if confirm.isEmpty { save() } else { asking = true } }
                .buttonStyle(.bordered)
                .disabled(action.busy)
        }
        .deviceConfirm(ActionConfirm(title: "Clear the sign-in?", text: confirm, confirm: "Clear"), isPresented: $asking) { save() }
    }
}

private struct DiagnosticsSection: View {
    let diagnostics: DiagnosticsSettings

    var body: some View {
        DeviceSection(title: "Remote diagnostics", badge: diagnostics.status) {
            DeviceText(text: diagnostics.hint)
            DiagnosticsForm(diagnostics: diagnostics).id("\(diagnostics.endpoint)|\(diagnostics.token)")
            DeviceSwitchRow(
                title: "Stream this device's logs",
                checked: diagnostics.streaming,
                on: DeviceRequest(method: .PUT, path: "/api/diagnostics", body: deviceBody(["enabled": "true"])),
                off: DeviceRequest(method: .PUT, path: "/api/diagnostics", body: deviceBody(["enabled": "false"])),
                success: "Streaming logs.",
                offSuccess: "Stopped streaming logs.",
                enabled: diagnostics.canStream,
                confirmOn: ActionConfirm(title: "Stream the logs?", text: diagnostics.confirm, confirm: "Stream")
            )
            DeviceStatusLine(text: diagnostics.state)
            DeviceStatusLine(text: "Ships this device's logs to an endpoint you name, so support can read them without the device in front of them.")
        }
    }
}

private struct DiagnosticsForm: View {
    let diagnostics: DiagnosticsSettings
    @Environment(\.LocalChangeNotice) private var notice
    @State private var action = DeviceActionState()
    @State private var endpoint: String
    @State private var token: String

    init(diagnostics: DiagnosticsSettings) {
        self.diagnostics = diagnostics
        _endpoint = State(initialValue: diagnostics.endpoint)
        _token = State(initialValue: diagnostics.token)
    }

    private func refused(_ field: String) -> Bool { action.refusal?.field == field }

    private func save() {
        let request = DeviceRequest(method: .POST, path: "/api/diagnostics/config", body: deviceBody(["endpoint": endpoint, "token": token]))
        let notice = notice, action = action
        Task { @MainActor in _ = await action.run(request, success: "Diagnostics endpoint saved.", notice: notice) }
    }

    var body: some View {
        DeviceTextField(
            label: "OTLP endpoint (host:port)",
            value: endpoint,
            onValueChange: { endpoint = $0 },
            placeholder: "otel.example.com:443",
            isError: refused("endpoint"),
            supporting: refused("endpoint") ? action.refusal?.message ?? "" : "",
            keyboard: .uri
        )
        SecretField(
            label: diagnostics.tokenLabel,
            value: token,
            onValueChange: { token = $0 },
            placeholder: "none",
            isError: refused("token"),
            supportingText: refused("token") ? action.refusal?.message ?? "" : ""
        )
        if let refusal = action.refusal, !refused("endpoint"), !refused("token") {
            DeviceRefusalText(refusal: refusal, field: refusal.field)
        }
        HStack(spacing: Space.s2) {
            Text("Stored on the device only.").font(.footnote).foregroundStyle(Color.secondary).frame(maxWidth: .infinity, alignment: .leading)
            Button("Save", action: save).buttonStyle(.bordered).disabled(action.busy)
        }
    }
}
