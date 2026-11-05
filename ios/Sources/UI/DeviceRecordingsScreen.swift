import SwiftUI

struct DeviceRecordingsScreen: View {
    @DeviceView("recordings") private var reading

    var body: some View {
        NativeForm {
            if let view = deviceView(reading, parseRecordings) {
                Section {
                    LimitPicker(limit: view.limit).id(view.limit.value)
                } header: {
                    SectionHeader(text: "Storage")
                } footer: {
                    Text("Recordings share the card this device boots from. Past the limit the oldest go first, so recording can never fill the card.")
                }
                if !view.cameras.isEmpty {
                    DeviceSection(title: "Cameras") {
                        ForEach(view.cameras, id: \.name) { camera in
                            DeviceSwitchRow(
                                title: camera.name,
                                checked: camera.recording,
                                on: DeviceRequest(method: .PUT, path: recordPath(camera.name)),
                                off: DeviceRequest(method: .DELETE, path: recordPath(camera.name)),
                                detail: camera.note,
                                success: "Recording turned on for \(camera.name).",
                                offSuccess: "Recording turned off for \(camera.name) — its recordings stay below."
                            )
                        }
                    }
                }
                if !view.recorded.isEmpty {
                    ForEach(view.recorded, id: \.name) { RecordedSection(camera: $0) }
                } else {
                    DeviceSection(title: "On this device") { DeviceText(text: "Nothing recorded yet. Turn recording on for a camera above.") }
                }
            } else {
                FootNote(text: DEVICE_WAITING)
            }
        }
    }
}

private struct LimitPicker: View {
    let limit: RecordingsLimit
    @Environment(\.LocalChangeNotice) private var notice
    @State private var action = DeviceActionState()
    @State private var picked: String
    @State private var lowering = ""
    @State private var asking = false

    init(limit: RecordingsLimit) {
        self.limit = limit
        _picked = State(initialValue: limit.value)
    }

    private func save(_ value: String) {
        picked = value
        let request = DeviceRequest(method: .PUT, path: "/api/recordings/limits", body: deviceBody(["maxBytes": value]))
        let notice = notice, action = action, previous = limit.value
        Task { @MainActor in
            if !(await action.run(request, success: "Recording limit saved.", notice: notice)) { picked = previous }
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            DeviceSelect(
                label: "Recordings may use",
                value: picked,
                options: limit.options,
                onPick: { value in
                    if lowersLimit(limit.value, value) {
                        lowering = value
                        asking = true
                    } else {
                        save(value)
                    }
                },
                enabled: !action.busy
            )
            if !limit.used.isEmpty {
                Text(limit.used).font(.footnote).foregroundStyle(Color.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .deviceConfirm(
            ActionConfirm(title: "Lower the limit?", text: "Lowering the limit deletes the oldest recordings straight away.", confirm: "Lower it"),
            isPresented: $asking
        ) { save(lowering) }
        DeviceRefusalText(refusal: action.refusal, field: action.refusal?.field ?? "")
    }
}

private struct RecordedSection: View {
    let camera: RecordedCamera

    var body: some View {
        DeviceSection(title: camera.name, badge: camera.status) {
            if camera.full {
                DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Warn, text: "Recordings have reached the size limit, so the oldest are being deleted to make room for new ones."))
            }
            HStack(spacing: Space.s2) {
                Text("\(camera.summary) · \(camera.usage)").font(.footnote).foregroundStyle(Color.secondary).frame(maxWidth: .infinity, alignment: .leading)
                DeviceDestructiveButton(
                    label: "Delete all",
                    request: DeviceRequest(method: .DELETE, path: recordingsPath(camera.name)),
                    success: "Deleted every recording of \(camera.name).",
                    confirm: ActionConfirm(title: "Delete every recording?", text: "Delete every recording of \(camera.name)? This cannot be undone.", confirm: "Delete all")
                )
            }
            ForEach(camera.segments, id: \.at) { SegmentRow(camera: camera.name, segment: $0) }
            DeviceStatusLine(
                text: "Each download carries the aircraft's height, speed, battery and position as a subtitle track. Players keep subtitles off until asked — in QuickTime choose View → Subtitles, " +
                    "in VLC Subtitle → Sub Track."
            )
        }
    }
}

private struct SegmentRow: View {
    let camera: String
    let segment: RecordingSegment
    @Environment(\.LocalChangeNotice) private var notice
    @Environment(\.LocalDeviceOrigin) private var origin
    @State private var action = DeviceActionState()
    @State private var asking = false

    private func delete() {
        let request = DeviceRequest(method: .DELETE, path: segmentPath(camera, segment))
        let notice = notice, action = action
        Task { @MainActor in _ = await action.run(request, success: "Recording deleted.", notice: notice) }
    }

    // One recording: its start and details, Download, and a red trash button.
    var body: some View {
        HStack(spacing: Space.s2) {
            VStack(alignment: .leading, spacing: 2) {
                Text(segment.started)
                Text(segmentDetail(segment).joined(separator: " · ")).font(.footnote).foregroundStyle(Color.secondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if !segment.open {
                LinkButton(href: resolveDeviceUrl(origin, segment.url), text: "Download", look: .Text, icon: .download)
            }
            Button(role: .destructive) { asking = true } label: {
                Image(systemName: "trash").frame(width: Metrics.iconButton, height: Metrics.iconButton).contentShape(Rectangle()).macHover(Circle())
            }
            .buttonStyle(.borderless)
            .disabled(action.busy)
            .accessibilityLabel("Delete the recording from \(segment.started)")
            .macTooltip("Delete the recording from \(segment.started)")
        }
        .deviceConfirm(
            ActionConfirm(title: "Delete this recording?", text: "Delete the recording from \(segment.started)?", confirm: "Delete"),
            isPresented: $asking,
            onConfirm: delete
        )
        DeviceRefusalText(refusal: action.refusal, field: action.refusal?.field ?? "")
    }
}
