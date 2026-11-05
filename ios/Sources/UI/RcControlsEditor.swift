import SwiftUI

let RC_CONTROLS = "settings.flyViewSettings.rcControls"

private let CAMERA_CHANNEL_SETTINGS: [(KeyPath<RcCameraChannels, Int>, String)] = [
    (\.tilt, "Gimbal tilt"),
    (\.pan, "Gimbal pan"),
    (\.zoom, "Camera zoom"),
    (\.light, "Camera light"),
    (\.record, "Camera record"),
]

private let UNDO_WINDOW_MS = 6000
private let RC_CONTROLS_TITLE = "On-screen RC controls"

private struct Draft: Equatable, Identifiable {
    let index: Int
    var label: String
    var channel: String
    var type: RcControlType

    var id: Int { index }
}

private struct Undo: Equatable {
    let name: String
    let previous: String
}

func reservedChannels(_ channels: RcCameraChannels) -> [Int: String] {
    Dictionary(CAMERA_CHANNEL_SETTINGS.map { field, owner in (channels[keyPath: field], owner) }, uniquingKeysWith: { $1 })
        .filter { $0.key > 0 }
}

struct RcControlsEditor: View {
    var heading: String = ""
    @QgcString(settingControl(RC_CONTROLS)) private var json
    @CameraChannels private var cameraChannels
    @State private var draft: Draft?
    @State private var notice: String?
    @State private var undo: Undo?
    @Environment(\.theme) private var theme

    private func save(_ next: String) {
        notice = nil
        offMain {
            if !Qgc.set(RC_CONTROLS, next) {
                onMain { notice = "The setting would not take that list." }
            }
        }
    }

    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            native
        } else {
            handDrawn
        }
    }

    private func edit(_ index: Int, _ control: RcControl) {
        draft = Draft(index: index, label: control.label, channel: String(control.channel), type: control.type)
    }

    private func remove(_ index: Int, _ control: RcControl) {
        undo = Undo(name: control.label, previous: json)
        save(rcControlsRemoved(json, index))
    }

    // Inside a NativeForm: a section with a row per control (tap to edit, swipe or right-click to remove), Undo after
    // a removal, and Add as the last row, which also carries the undo window and the editor sheet.
    @ViewBuilder private var native: some View {
        let controls = parseRcControls(json)
        let reserved = reservedChannels(cameraChannels)
        Section {
            if let notice {
                Text(notice).foregroundStyle(theme.colors.error)
            }
            if controls.isEmpty {
                Text("No on-screen controls yet. Add one to drive a channel from the Fly view.").foregroundStyle(Color.secondary)
            }
            ForEach(Array(controls.enumerated()), id: \.offset) { index, control in
                let clash = channelOwner(json, control.channel, index, reserved)
                Button { edit(index, control) } label: {
                    NativeRowLabel(title: control.label, subtitle: controlSubtitle(control, clash))
                }
                .listRowButton()
                .swipeActions {
                    Button("Remove", role: .destructive) { remove(index, control) }
                }
                .contextMenu {
                    Button("Edit") { edit(index, control) }
                    Button("Remove", role: .destructive) { remove(index, control) }
                }
            }
            if let undo {
                LabeledContent("Removed \(undo.name).") {
                    Button("Undo") {
                        save(undo.previous)
                        self.undo = nil
                    }
                }
            }
            Button("Add control") {
                draft = Draft(index: -1, label: "", channel: String(firstFreeChannel(json, reserved)), type: .Slider)
            }
            .task(id: undo) { await expireUndo() }
            .sheet(item: $draft) { opened in
                DraftDialog(initial: opened, json: json, reserved: reserved, onDismiss: { draft = nil }) { label, channel, type, index in
                    save(index < 0 ? rcControlsAdded(json, label, channel, type) : rcControlsPatched(json, index, label, channel, type))
                    draft = nil
                }
            }
        } header: {
            SectionHeader(text: heading.ifBlank(RC_CONTROLS_TITLE))
        }
    }

    private func controlSubtitle(_ control: RcControl, _ clash: String?) -> Text {
        let line = Text("Channel \(control.channel) · \(typeLabel(control.type))")
        guard let clash else { return line }
        return line + Text("\nChannel \(control.channel) is already driving \(clash).").foregroundStyle(theme.colors.error)
    }

    @MainActor private func expireUndo() async {
        guard undo != nil else { return }
        try? await Task.sleep(for: .milliseconds(UNDO_WINDOW_MS))
        if !Task.isCancelled { undo = nil }
    }

    private var handDrawn: some View {
        let controls = parseRcControls(json)
        let reserved = reservedChannels(cameraChannels)
        return VStack(alignment: .leading, spacing: 0) {
            SectionHeader(text: RC_CONTROLS_TITLE)
            if let notice {
                Text(notice)
                    .font(.bodyMedium)
                    .foregroundStyle(theme.colors.error)
                    .padding(Space.s4)
            }
            if controls.isEmpty {
                Text("No on-screen controls yet. Add one to drive a channel from the Fly view.")
                    .padding(Space.s4)
            }
            ForEach(Array(controls.enumerated()), id: \.offset) { index, control in
                let clash = channelOwner(json, control.channel, index, reserved)
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(control.label).font(.bodyLarge)
                        Text("Channel \(control.channel) · \(typeLabel(control.type))").font(.bodySmall)
                        if let clash {
                            Text("Channel \(control.channel) is already driving \(clash).")
                                .font(.bodySmall)
                                .foregroundStyle(theme.colors.error)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    Button("Edit") {
                        draft = Draft(index: index, label: control.label, channel: String(control.channel), type: control.type)
                    }
                    .buttonStyle(.text)
                    Button("Remove") {
                        undo = Undo(name: control.label, previous: json)
                        save(rcControlsRemoved(json, index))
                    }
                    .buttonStyle(.text)
                }
                .padding(.horizontal, Space.s4)
                .padding(.vertical, 10)
                Divider()
            }
            if let undo {
                HStack {
                    Text("Removed \(undo.name).").font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
                    Button("Undo") {
                        save(undo.previous)
                        self.undo = nil
                    }
                    .buttonStyle(.text)
                }
                .padding(.horizontal, Space.s4)
                .padding(.vertical, Space.s1)
            }
            Button("Add control") {
                draft = Draft(index: -1, label: "", channel: String(firstFreeChannel(json, reserved)), type: .Slider)
            }
            .buttonStyle(.filled)
            .padding(Space.s4)
        }
        .task(id: undo) { await expireUndo() }
        .sheet(item: $draft) { opened in
            DraftDialog(initial: opened, json: json, reserved: reserved, onDismiss: { draft = nil }) { label, channel, type, index in
                save(index < 0 ? rcControlsAdded(json, label, channel, type) : rcControlsPatched(json, index, label, channel, type))
                draft = nil
            }
        }
    }
}

private struct DraftDialog: View {
    let json: String
    let reserved: [Int: String]
    let onDismiss: () -> Void
    let onSave: (String, Int, RcControlType, Int) -> Void
    @State private var draft: Draft
    @Environment(\.theme) private var theme

    init(initial: Draft, json: String, reserved: [Int: String], onDismiss: @escaping () -> Void, onSave: @escaping (String, Int, RcControlType, Int) -> Void) {
        self.json = json
        self.reserved = reserved
        self.onDismiss = onDismiss
        self.onSave = onSave
        _draft = State(initialValue: initial)
    }

    var body: some View {
        let channel = Int(draft.channel) ?? 0
        let owner = channelOwner(json, channel, draft.index, reserved)
        NavigationStack {
            Form {
                TextField("Name", text: $draft.label)
                TextField("Channel", text: Binding(
                    get: { draft.channel },
                    set: { typed in draft.channel = String(typed.filter { $0.isASCII && $0.isNumber }.prefix(2)) }
                ))
                .keyboardType(.numberPad)
                if !channelUsable(channel) {
                    Text("Channels run from \(RC_CHANNEL_MIN) to \(RC_CHANNEL_MAX).")
                        .font(.bodySmall)
                        .foregroundStyle(theme.colors.error)
                }
                if let owner {
                    Text("Channel \(channel) is already driving \(owner).")
                        .font(.bodySmall)
                        .foregroundStyle(theme.colors.error)
                }
                Picker("Type", selection: $draft.type) {
                    ForEach(RcControlType.allCases, id: \.self) { Text(typeLabel($0)).tag($0) }
                }
                .pickerStyle(.inline)
            }
            .navigationTitle(draft.index < 0 ? "New control" : "Edit control")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("Cancel", action: onDismiss) }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") { onSave(draft.label.ifBlank("CH\(channel)"), channel, draft.type, draft.index) }
                        .disabled(!(channelUsable(channel) && owner == nil))
                }
            }
        }
        .presentationDetents([.medium, .large])
        .presentationDragIndicator(.visible)
    }
}
