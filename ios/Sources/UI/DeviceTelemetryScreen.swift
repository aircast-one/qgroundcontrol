import SwiftUI

struct DeviceTelemetryScreen: View {
    let props: DeviceScreenProps
    @DeviceView("telemetry") private var reading

    var body: some View {
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

private struct FixSteps: View {
    let fix: TelemetryFix
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(fix.title).font(.titleMedium)
            ForEach(Array(fix.steps.enumerated()), id: \.offset) { index, step in
                HStack(alignment: .top, spacing: 12) {
                    Text("\(index + 1)")
                        .font(.labelMedium)
                        .foregroundStyle(theme.colors.onPrimaryContainer)
                        .frame(width: 24, height: 24)
                        .background(theme.colors.primaryContainer, in: Circle())
                    Text(step).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            if !fix.waiting.isEmpty {
                HStack(spacing: 12) {
                    ProgressView().controlSize(.small)
                    Text(fix.waiting).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                }
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: 12))
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

private struct CloudPath: View {
    let cloud: TelemetryCloud
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 12) {
                Text("Cloud path").font(.bodyLarge).frame(maxWidth: .infinity, alignment: .leading)
                DeviceBadgeText(badge: cloud.status)
            }
            Text(cloud.text).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
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
        Group {
            if let editor {
                form(editor)
            }
        }
        .task(id: typed) {
            let loaded = await endpointEditor(view.schemes, typed, view.settings.endpoints)
            if !Task.isCancelled { editor = loaded }
        }
    }

    private func form(_ editor: EndpointEditor) -> some View {
        let rows = typed ?? editor.rows.map(\.row)
        let baudRefused = action.refusal?.field == "baud"
        return VStack(alignment: .leading, spacing: 12) {
            Text("Ground-station endpoints").font(.bodyLarge)
            ForEach(Array(rows.enumerated()), id: \.offset) { index, row in
                EndpointEditorRow(
                    row: row,
                    shown: editor.rows.indices.contains(index) ? editor.rows[index] : editor.blank,
                    onChange: { next in typed = rows.enumerated().map { at, it in at == index ? next : it } },
                    onRemove: { typed = rows.enumerated().filter { $0.offset != index }.map(\.element) }
                )
            }
            Button("+ Add endpoint") { typed = rows + [editor.blank.row] }.buttonStyle(.text)
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
            Text("Takes effect on save — no reboot.").font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            if let refusal = action.refusal, !baudRefused {
                Text(refusal.message).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
            Button("Save") { save(rows) }.buttonStyle(.bordered).disabled(action.busy)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

private struct EndpointEditorRow: View {
    let row: EndpointRow
    let shown: EndpointRowView
    let onChange: (EndpointRow) -> Void
    let onRemove: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .bottom, spacing: 8) {
                DeviceSelect(
                    label: "Connection type",
                    value: row.scheme,
                    options: shown.options.map { DeviceOption(value: $0.value, label: $0.label) },
                    onPick: { scheme in onChange(shown.options.first { $0.value == scheme }?.row ?? row) }
                )
                Button(action: onRemove) { Image(.close).frame(width: 48, height: 48).contentShape(Rectangle()) }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Remove endpoint")
            }
            HStack(alignment: .top, spacing: 8) {
                DeviceTextField(
                    label: shown.hostLabel,
                    value: row.host,
                    onValueChange: { host in onChange(withChanges(row) { $0.host = host }) },
                    placeholder: shown.placeholder,
                    keyboard: .uri
                )
                DeviceTextField(
                    label: "Port",
                    value: row.port,
                    onValueChange: { port in onChange(withChanges(row) { $0.port = port }) },
                    placeholder: "14550",
                    keyboard: .number
                )
                .frame(width: 112)
            }
            if !shown.problem.isEmpty {
                Text(shown.problem).font(.bodySmall).foregroundStyle(theme.colors.error)
            } else {
                Text(shown.note).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
        .padding(12)
        .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.medium))
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
