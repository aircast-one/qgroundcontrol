import SwiftUI

struct ConnectionsSection: View {
    @ObservedObject var store: LinksStore
    @State private var confirmingRemoval: Int?
    @State private var newType = 0
    @State private var newName = ""
    @State private var newHost = ""
    @State private var newPort = "5760"
    @State private var addError = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("Links").font(.headline)
                Spacer()
                Button(store.adding ? "Cancel" : "Add Link…") { store.adding.toggle() }
            }
            .padding(.bottom, 8)

            if store.adding { addForm }

            if store.links.isEmpty {
                Text("No links configured.")
                    .foregroundColor(.secondary)
                    .padding(.vertical, 8)
            } else {
                ForEach(Array(store.links.enumerated()), id: \.element.id) { position, link in
                    if position > 0 { Divider() }
                    row(link)
                    if store.editingIndex == link.id { editor(link) }
                }
            }
        }
        .onAppear(perform: store.startPolling)
        .onDisappear(perform: store.stopPolling)
    }

    // Without this an operator with a new vehicle cannot get connected at all: every
    // other control here edits a link that already exists.
    private var addForm: some View {
        VStack(alignment: .leading, spacing: 10) {
            Picker("Type", selection: $newType) {
                ForEach(Array(store.linkTypes.enumerated()), id: \.offset) { index, label in
                    Text(label).tag(index)
                }
            }
            LabelledField(label: "Name", value: newName) { newName = $0 }
            if store.linkTypes.indices.contains(newType), store.linkTypes[newType] == "Serial" {
                LabelledPicker(label: "Port",
                               options: store.serialPorts.map { ($0.label, $0.device) },
                               selection: newHost) { newHost = $0 }
                LabelledPicker(label: "Baud",
                               options: store.baudRates.map { (String($0), String($0)) },
                               selection: newPort.isEmpty ? "57600" : newPort) { newPort = $0 }
            } else {
                LabelledField(label: "Host", value: newHost) { newHost = $0 }
                LabelledField(label: "Port", value: newPort) { newPort = $0 }
            }
            if !addError.isEmpty {
                Text(addError).font(.caption).foregroundColor(.red)
            }
            HStack {
                Spacer()
                Button("Create") { create() }
                    .disabled(newName.trimmingCharacters(in: .whitespaces).isEmpty)
            }
        }
        .padding(.vertical, 10)
    }

    private func create() {
        guard !newName.trimmingCharacters(in: .whitespaces).isEmpty else {
            addError = "Give the link a name."
            return
        }
        // Int("banana") ?? 0 would have created a link on port 0 and failed silently
        // at connect time, far from the mistake.
        guard let port = Int(newPort), (1...65535).contains(port) else {
            addError = "Port must be between 1 and 65535."
            return
        }
        guard store.create(type: newType, name: newName, host: newHost, port: port) else {
            addError = "Could not create the link."
            return
        }
        addError = ""
        newName = ""
        store.adding = false
    }

    private func row(_ link: LinkConfig) -> some View {
        HStack(spacing: 12) {
            Circle()
                .fill(link.connected ? Color.green : Color.secondary.opacity(0.35))
                .frame(width: 8, height: 8)

            VStack(alignment: .leading, spacing: 2) {
                Text(link.name)
                Text("\(link.typeLabel) · \(link.displaySummary)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                if !link.lastError.isEmpty {
                    Text(link.lastError)
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }
            Spacer()

            if store.connectingName == link.name {
                ProgressView().controlSize(.small)
            }
            Button(link.connected ? "Disconnect" : "Connect") {
                link.connected ? store.disconnect(link) : store.connect(link)
            }
            Button(store.editingIndex == link.id ? "Done" : "Edit") {
                store.editingIndex = store.editingIndex == link.id ? nil : link.id
            }
        }
        .padding(.vertical, 7)
    }

    private func portField(_ link: LinkConfig) -> some View {
        LabelledField(label: "Port", value: String(link.port)) {
            if let port = Int($0), (1...65535).contains(port) { store.setPort(link, port) }
        }
    }

    @ViewBuilder
    private func editor(_ link: LinkConfig) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            LabelledField(label: "Name", value: link.name) { store.rename(link, to: $0) }
            switch link.editing {
            case .hostAndPort:
                LabelledField(label: "Host", value: link.host) { store.setHost(link, $0) }
                portField(link)
            case .portOnly:
                portField(link)
            case .serial:
                LabelledPicker(label: "Port",
                               options: store.serialPorts.map { ($0.label, $0.device) },
                               selection: link.portName) { store.setPortName(link, $0) }
                LabelledPicker(label: "Baud",
                               options: store.baudRates.map { (String($0), String($0)) },
                               selection: String(link.baud)) { store.setBaud(link, Int($0) ?? link.baud) }
            case .none:
                EmptyView()
            }
            Toggle("Connect automatically on start", isOn: Binding(
                get: { link.autoConnect },
                set: { store.setAutoConnect(link, $0) }))

            HStack {
                Spacer()
                if confirmingRemoval == link.id {
                    Text("Delete this link?").font(.caption).foregroundColor(.secondary)
                    Button("Cancel") { confirmingRemoval = nil }
                    Button("Delete") {
                        confirmingRemoval = nil
                        store.editingIndex = nil
                        store.remove(link)
                    }
                } else {
                    // Deleting a link an operator relies on is not recoverable, so it
                    // asks first rather than relying on an undo that does not exist.
                    Button("Delete Link…") { confirmingRemoval = link.id }
                        .disabled(link.connected)
                        .help(link.connected ? "Disconnect before deleting" : "")
                }
            }
        }
        .padding(.leading, 20)
        .padding(.vertical, 10)
    }
}

struct LabelledField: View {
    let label: String
    let value: String
    let commit: (String) -> Void

    @State private var draft = ""
    @FocusState private var editing: Bool

    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
                .frame(width: 60, alignment: .leading)
            TextField("", text: $draft)
                .onAppear { draft = value }
                .onChange(of: value) { latest in if !editing { draft = latest } }
                .focused($editing)
                .onSubmit { commit(draft) }
                .onChange(of: editing) { focused in if !focused { commit(draft) } }
        }
    }
}


struct LabelledPicker: View {
    let label: String
    let options: [(String, String)]
    let selection: String
    let commit: (String) -> Void

    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
                .frame(width: 60, alignment: .leading)
            Picker("", selection: Binding(get: { selection }, set: commit)) {
                if !options.contains(where: { $0.1 == selection }) {
                    Text(selection.isEmpty ? "Not set" : selection).tag(selection)
                }
                ForEach(options, id: \.1) { option in
                    Text(option.0).tag(option.1)
                }
            }
            .labelsHidden()
        }
    }
}
