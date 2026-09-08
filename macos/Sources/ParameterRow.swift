import SwiftUI

struct ValueField: View {
    let value: String
    let units: String
    let commit: (String) -> Void

    @State private var draft = ""
    @FocusState private var editing: Bool

    var body: some View {
        HStack(spacing: 3) {
            TextField("", text: $draft)
                .textFieldStyle(.plain)
                .multilineTextAlignment(.trailing)
                .font(.body.monospacedDigit())
                .frame(width: 72)
                .focused($editing)
                .onAppear { draft = value }
                .onChange(of: value) { latest in if !editing { draft = latest } }
                .onSubmit(send)
                .onChange(of: editing) { focused in
                    guard !focused else { return }
                    send()
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
        guard trimmed != value else { return }
        guard !trimmed.isEmpty else {
            draft = value
            return
        }
        commit(trimmed)
    }
}

struct ParameterEditor: View {
    let parameter: Parameter
    let commit: (String) -> Void

    @ViewBuilder var body: some View {
        if parameter.options.isEmpty {
            ValueField(value: parameter.value, units: parameter.units, commit: commit)
        } else {
            Picker("", selection: Binding(
                get: { parameter.selectedOption?.raw ?? "" },
                set: { commit($0) })
            ) {
                if parameter.selectedOption == nil {
                    Text(parameter.value).tag("")
                }
                ForEach(parameter.options) { option in
                    Text(option.label).tag(option.raw)
                }
            }
            .labelsHidden()
            .frame(maxWidth: 220)
        }
    }
}

struct ParameterRow: View {
    let parameter: Parameter
    var showSeparator = true
    let commit: (String) -> Void

    var body: some View {
        GroupRow(
            title: parameter.description.isEmpty ? parameter.name : parameter.description,
            description: parameter.description.isEmpty ? "" : parameter.name,
            showSeparator: showSeparator,
            trailing: { ParameterEditor(parameter: parameter, commit: commit) })
    }
}

struct SetupPageBody<Content: View>: View {
    let title: String
    var note = ""
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
                content
            }
            .padding(Overlay.unit * 1.25)
            .frame(maxWidth: 660, alignment: .leading)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
