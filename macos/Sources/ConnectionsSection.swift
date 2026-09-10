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

    static func tone(_ health: LinkConfig.Health) -> Color {
        switch health {
        case .closed: return .secondary.opacity(0.35)
        case .waiting: return .orange
        case .heard: return .green
        }
    }

    private func row(_ link: LinkConfig) -> some View {
        HStack(spacing: 12) {
            Circle()
                .fill(ConnectionsSection.tone(link.health))
                .frame(width: 8, height: 8)

            VStack(alignment: .leading, spacing: 2) {
                Text(link.name)
                Text("\(link.typeLabel) · \(link.statusLine)")
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
            case .logFile:
                HStack(spacing: Overlay.step) {
                    Text("Log file")
                        .frame(width: 96, alignment: .leading)
                    Text(link.filename.isEmpty ? "No log chosen" : link.logFileName)
                        .foregroundColor(link.filename.isEmpty ? .secondary : Overlay.value)
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .help(link.filename)
                    Spacer()
                    Button("Choose\u{2026}") { store.chooseLogFile(link) }
                }
            case .serial:
                LabelledPicker(label: "Port",
                               options: store.serialPorts.map { ($0.label, $0.device) },
                               selection: link.portName) { store.setPortName(link, $0) }
                LabelledPicker(label: "Baud",
                               options: store.baudRates.map { (String($0), String($0)) },
                               selection: String(link.baud)) { store.setBaud(link, Int($0) ?? link.baud) }
            case .none, .unknown:
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



