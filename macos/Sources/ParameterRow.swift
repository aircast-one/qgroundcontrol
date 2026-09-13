import SwiftUI

struct ValueField: View {
    let value: String
    let units: String
    var width = 72.0
    let commit: (String) -> Void

    @State private var draft = ""
    @FocusState private var editing: Bool

    var body: some View {
        HStack(spacing: 3) {
            TextField("", text: Binding(
                get: { EditedField.shown(typed: draft, held: value, editing: editing) },
                set: { draft = $0 }))
                .textFieldStyle(.plain)
                .multilineTextAlignment(.trailing)
                .font(.body.monospacedDigit())
                .frame(width: width)
                .focused($editing)
                .onSubmit { editing = false }
                .onChange(of: editing) { focused in
                    if focused { draft = value } else { send() }
                }
            if !units.isEmpty {
                Text(units).font(.caption).foregroundColor(.secondary).fixedSize()
            }
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 2)
        .background(Color.primary.opacity(editing ? 0.10 : 0.06))
        .cornerRadius(6)
    }

    private func send() {
        let trimmed = draft.trimmingCharacters(in: .whitespaces)
        guard trimmed != value, !trimmed.isEmpty else { return }
        commit(trimmed)
    }
}

struct ParameterEditor: View {
    let value: String
    let units: String
    let options: [ParameterOption]
    let selectedRaw: String
    let commit: (String) -> Void

    @ViewBuilder var body: some View {
        if options.isEmpty {
            ValueField(value: value, units: units, commit: commit)
        } else {
            Picker("", selection: Binding(get: { selectedRaw }, set: { commit($0) })) {
                if !options.contains(where: { $0.raw == selectedRaw }) {
                    Text(value).tag(selectedRaw)
                }
                ForEach(options) { option in
                    Text(option.label).tag(option.raw)
                }
            }
            .labelsHidden()
            .frame(maxWidth: 220)
        }
    }
}

struct ParameterRow: View {
    let name: String
    let label: String
    var detail = ""
    let value: String
    let units: String
    let options: [ParameterOption]
    let selectedRaw: String
    var showSeparator = true
    let commit: (String) -> Void

    init(parameter: Parameter, showSeparator: Bool = true,
         commit: @escaping (String) -> Void) {
        name = parameter.name
        label = parameter.description
        value = parameter.value
        detail = label.isEmpty ? "" : parameter.name
        units = parameter.units
        options = parameter.options
        selectedRaw = parameter.selectedOption?.raw ?? ""
        self.showSeparator = showSeparator
        self.commit = commit
    }

    init(control: SettingsControl, showSeparator: Bool = true,
         commit: @escaping (String) -> Void) {
        detail = control.rowDescription(label: control.label)
        name = control.name
        label = control.label
        value = control.display.isEmpty ? control.valueString : control.display
        units = control.units
        options = control.parameterOptions
        selectedRaw = control.valueString
        self.showSeparator = showSeparator
        self.commit = commit
    }

    var body: some View {
        GroupRow(
            title: label.isEmpty ? name : label,
            description: detail,
            showSeparator: showSeparator,
            trailing: {
                ParameterEditor(value: value, units: units, options: options,
                                selectedRaw: selectedRaw, commit: commit)
            })
    }
}

struct SetupPageBody<Content: View>: View {
    let title: String
    var note = ""
    var connected = true
    @ViewBuilder var content: Content

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Overlay.unit * 1.25) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(title).font(.title2.weight(.semibold))
                    if !note.isEmpty {
                        Text(note).font(.callout).foregroundColor(.secondary)
                    }
                }
                if connected {
                    content
                } else {
                    GroupCard {
                        EmptyStateRow(text: "Connect a vehicle to set this up.")
                    }
                }
            }
            .padding(Overlay.unit * 1.25)
            .frame(maxWidth: 660, alignment: .leading)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct SetupSections: View {
    let sections: [SettingsSection]
    @ObservedObject var store: ParametersStore

    var body: some View {
        ForEach(sections) { section in
            VStack(alignment: .leading, spacing: 0) {
                SectionLabel(text: section.title)
                GroupCard {
                    ForEach(Array(section.controls.enumerated()), id: \.element.id) { index, control in
                        ParameterRow(control: control, showSeparator: index > 0) {
                            store.writeControl(control, $0)
                        }
                    }
                }
                if !section.note.isEmpty {
                    Text(section.note)
                        .font(.caption).foregroundColor(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                        .padding(.horizontal, Overlay.horizontalPadding)
                        .padding(.top, Overlay.unit * 0.35)
                }
            }
        }
    }
}
