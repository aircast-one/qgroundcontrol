import SwiftUI

struct DeviceTelemetryScreen: View {
    let props: DeviceScreenProps
    @DeviceView("telemetry") private var reading

    var body: some View {
        NativeForm {
            if let view = deviceView(reading, parseTelemetry) {
                DeviceSection(title: "Status", badge: view.status) {
                    if let fix = view.fix { FixSteps(fix: fix) }
                    ForEach(view.notices, id: \.text) { DeviceNotice(notice: $0) }
                    if let cloud = view.cloud { CloudPath(cloud: cloud) }
                }
                DeviceSection(title: "Connect another ground station") {
                    EndpointRows(endpoints: view.endpoints)
                    if let notice = view.endpointNotice { DeviceNotice(notice: notice) }
                }
                DeviceSection(title: "MAVLink settings") {
                    MavlinkEditor(view: view).id(view.settings)
                }
                if let gimbal = view.gimbal { GimbalSection(gimbal: gimbal) }
            } else {
                FootNote(text: DEVICE_WAITING)
            }
        }
    }
}

private struct FixSteps: View {
    let fix: TelemetryFix
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s3) {
            Text(fix.title).font(.headline)
            ForEach(Array(fix.steps.enumerated()), id: \.offset) { index, step in
                HStack(alignment: .top, spacing: Space.s3) {
                    Text("\(index + 1)")
                        .font(.footnote.weight(.semibold))
                        .foregroundStyle(theme.colors.onPrimaryContainer)
                        .frame(width: 24, height: 24)
                        .background(theme.colors.primaryContainer, in: Circle())
                    Text(step).frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            if !fix.waiting.isEmpty {
                HStack(spacing: Space.s3) {
                    ProgressView().controlSize(.small)
                    Text(fix.waiting).font(.footnote).foregroundStyle(Color.secondary)
                }
            }
        }
        .padding(.vertical, Space.s1)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct CloudPath: View {
    let cloud: TelemetryCloud

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: Space.s3) {
                Text("Cloud path").frame(maxWidth: .infinity, alignment: .leading)
                DeviceBadgeText(badge: cloud.status)
            }
            Text(cloud.text).font(.subheadline).foregroundStyle(Color.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct MavlinkEditor: View {
    let view: TelemetryView
    @Environment(\.theme) private var theme
    @Environment(\.LocalChangeNotice) private var notice
    @State private var action = DeviceActionState()
    @State private var typed: [EndpointRow]?
    @State private var baud: String
    @State private var editor: EndpointEditor?

    init(view: TelemetryView) {
        self.view = view
        _baud = State(initialValue: view.settings.baud)
    }

    private func save(_ rows: [EndpointRow]) {
        let schemes = view.schemes, stored = view.settings.endpoints, baud = baud, notice = notice, action = action
        Task { @MainActor in
            guard let saved = await endpointEditor(schemes, rows, stored) else { return }
            _ = await action.run(
                DeviceRequest(method: .PUT, path: "/api/telemetry/config", body: deviceBody(["endpoints": saved.endpoints, "baud": baud])),
                success: "MAVLink settings applied.",
                notice: notice
            )
        }
    }

    var body: some View {
        let rows = editor.map { typed ?? $0.rows.map(\.row) }
        endpointList(rows)
            .task(id: typed) {
                let loaded = await endpointEditor(view.schemes, typed, view.settings.endpoints)
                if !Task.isCancelled { editor = loaded }
            }
        if let rows {
            let baudRefused = action.refusal?.field == "baud"
            DeviceTextField(
                label: "Serial baud",
                value: baud,
                onValueChange: { baud = $0 },
                placeholder: "115200",
                isError: baudRefused,
                supporting: baudRefused ? action.refusal?.message ?? ""
                    : "The baud only matters for a serial flight controller — USB autopilots ignore it, Betaflight ships at 115200.",
                keyboard: .number
            )
            if let refusal = action.refusal, !baudRefused {
                Text(refusal.message).font(.footnote).foregroundStyle(theme.colors.error)
            }
            VStack(alignment: .leading, spacing: Space.s2) {
                Button("Save") { save(rows) }.buttonStyle(.bordered).disabled(action.busy)
                Text("Takes effect on save — no reboot.").font(.footnote).foregroundStyle(Color.secondary)
            }
        }
    }

    @ViewBuilder
    private func endpointList(_ rows: [EndpointRow]?) -> some View {
        VStack(alignment: .leading, spacing: Space.s3) {
            Text("Ground-station endpoints").font(.headline)
            if let editor, let rows {
                ForEach(Array(rows.enumerated()), id: \.offset) { index, row in
                    if index > 0 { Divider() }
                    EndpointEditorRow(
                        row: row,
                        shown: editor.rows.indices.contains(index) ? editor.rows[index] : editor.blank,
                        onChange: { next in typed = rows.enumerated().map { at, it in at == index ? next : it } },
                        onRemove: { typed = rows.enumerated().filter { $0.offset != index }.map(\.element) }
                    )
                }
                Button {
                    typed = rows + [editor.blank.row]
                } label: {
                    Label("Add endpoint", systemImage: "plus.circle.fill")
                }
                .buttonStyle(.borderless)
            } else {
                ProgressView().frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, Space.s1)
    }
}

private struct EndpointEditorRow: View {
    let row: EndpointRow
    let shown: EndpointRowView
    let onChange: (EndpointRow) -> Void
    let onRemove: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            HStack(spacing: Space.s2) {
                DeviceSelect(
                    label: "Connection type",
                    value: row.scheme,
                    options: shown.options.map { DeviceOption(value: $0.value, label: $0.label) },
                    onPick: { scheme in onChange(shown.options.first { $0.value == scheme }?.row ?? row) }
                )
                DeviceRemoveButton(label: "Remove endpoint", action: onRemove)
            }
            if !shown.hostLabel.isEmpty {
                DeviceTextField(
                    label: shown.hostLabel,
                    value: row.host,
                    onValueChange: { host in onChange(withChanges(row) { $0.host = host }) },
                    placeholder: shown.placeholder,
                    keyboard: .uri
                )
            }
            DeviceTextField(
                label: "Port",
                value: row.port,
                onValueChange: { port in onChange(withChanges(row) { $0.port = port }) },
                placeholder: "14550",
                keyboard: .number
            )
            if !shown.problem.isEmpty {
                Text(shown.problem).font(.footnote).foregroundStyle(theme.colors.error)
            } else {
                Text(shown.note).font(.footnote).foregroundStyle(Color.secondary)
            }
        }
    }
}

private struct GimbalSection: View {
    let gimbal: TelemetryGimbal

    var body: some View {
        DeviceSection(title: gimbal.title, badge: gimbal.status) {
            DeviceText(text: gimbal.about)
            DeviceForm(
                fields: [
                    DeviceField(name: "vendor", label: "Make", value: gimbal.vendor, options: gimbal.vendors),
                    DeviceField(name: "model", label: "Model", value: gimbal.model, options: gimbal.models),
                    DeviceField(name: "cameraAddr", label: "Camera address", value: gimbal.cameraAddr, placeholder: "192.168.144.25:37260", keyboard: .uri),
                    DeviceField(name: "mavlinkAddr", label: "MAVLink endpoint", value: gimbal.mavlinkAddr, placeholder: "127.0.0.1:14550", keyboard: .uri),
                ],
                method: .PUT,
                path: "/api/gimbal/config",
                submit: "Save",
                success: "Gimbal settings applied.",
                note: "Takes effect on save — no reboot. Changing the make resets the model and address to that make's defaults if the old ones do not fit."
            )
            .id([gimbal.vendor, gimbal.model, gimbal.cameraAddr, gimbal.mavlinkAddr])
        }
    }
}
