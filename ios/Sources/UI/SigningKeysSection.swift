import SwiftUI
import UIKit
import UniformTypeIdentifiers

let MAVLINK_GROUP = "mavlinkSettings"
let SIGNING_AFTER_BLOCK = "Ground Station"
let SIGNING_KEYS_VIEW = "view.signingKeys"
let SIGNING_ADD_PASSPHRASE = "signingKeys.addPassphrase"
let SIGNING_ADD_RAW = "signingKeys.addRaw"
let SIGNING_GENERATE = "signingKeys.generate"
let SIGNING_REMOVE = "signingKeys.remove"
let SIGNING_EXPORT = "signingKeys.export"
let SIGNING_ENABLE = "signing.enable"
let SIGNING_DISABLE = "signing.disable"
private let SIGNING_POLL_MS = 500
let RAW_KEY_HEX_LENGTH = 64
private let CLIPBOARD_WIPE_MS = 30_000

struct SigningKeyRow: Equatable {
    var name: String
    var inUse: Bool
    var activeOnVehicle: Bool = false
}

struct SigningKeys: Equatable {
    var available: Bool
    var vehicle: Bool
    var armed: Bool
    var state: String
    var linkName: String
    var activeKey: String
    var minPassphraseLength: Int
    var keys: [SigningKeyRow]
}

func signingKeys(_ view: JSON?) -> SigningKeys? {
    guard let view, view["available"].bool else { return nil }
    return SigningKeys(
        available: true,
        vehicle: view["vehicle"].bool,
        armed: view["armed"].bool,
        state: view["state"].string,
        linkName: view["linkName"].string,
        activeKey: view["activeKey"].string,
        minPassphraseLength: view["minPassphraseLength"].int(8),
        keys: view["keys"].array.filter { $0.object != nil }.map {
            SigningKeyRow(name: $0["name"].string, inUse: $0["inUse"].bool, activeOnVehicle: $0["activeOnVehicle"].bool)
        }
    )
}

struct KeyButtons: Equatable {
    let enable: Bool
    let disable: Bool
    let otherActive: Bool
    let pending: Bool
}

func keyButtons(_ keys: SigningKeys, _ row: SigningKeyRow) -> KeyButtons {
    let anyActive = keys.vehicle && keys.activeKey != "None"
    return KeyButtons(
        enable: !anyActive,
        disable: keys.vehicle && row.activeOnVehicle,
        otherActive: anyActive && !row.activeOnVehicle,
        pending: keys.state == "enabling" || keys.state == "disabling"
    )
}

func isHexKey(_ text: String) -> Bool { text.count == RAW_KEY_HEX_LENGTH && text.allSatisfy(\.isHexDigit) }

func canAddKey(_ name: String, _ raw: Bool, _ secret: String, _ minPassphrase: Int) -> Bool {
    !name.isEmpty && (raw ? isHexKey(secret) : secret.count >= minPassphrase)
}

private func refusalOf(_ path: String, _ args: [Any?]) -> String? { refusal(Qgc.call(path, arguments: args)) }

private func textOf(_ json: JSON) -> String? {
    if case .string(let text) = json { return text }
    return nil
}

private func shown(_ value: Binding<String?>) -> Binding<Bool> {
    Binding(get: { value.wrappedValue != nil }, set: { if !$0 { value.wrappedValue = nil } })
}

private func copyForAWhile(_ hex: String) {
    UIPasteboard.general.setItems(
        [[UTType.plainText.identifier: hex]],
        options: [.expirationDate: Date().addingTimeInterval(Double(CLIPBOARD_WIPE_MS) / 1000), .localOnly: true]
    )
}

struct SigningKeysSection: View {
    @State private var revision = 0
    @State private var read: SigningKeys?
    @State private var adding = false
    @State private var confirmDelete: String?
    @State private var exported: String?
    @State private var confirmEnable: String?
    @State private var armedWarning = false
    @State private var refused: String?
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let keys = read {
                SectionHeader(text: "MAVLink 2 signing")
                section(keys)
            }
        }
        .task(id: revision) {
            read = await offMain { signingKeys(Qgc.get(SIGNING_KEYS_VIEW)) }
            if read?.state == "enabling" || read?.state == "disabling" {
                try? await Task.sleep(for: .milliseconds(SIGNING_POLL_MS))
                if !Task.isCancelled { revision += 1 }
            }
        }
        .alert("Send signing key", isPresented: shown($confirmEnable), presenting: confirmEnable) { name in
            Button("OK") { change(SIGNING_ENABLE, name) }
            Button("Cancel", role: .cancel) {}
        } message: { name in
            Text("This will transmit key '\(name)' to the vehicle over '\(read?.linkName ?? "")'. Only proceed if this link is secure (USB or trusted local network).")
        }
        .alert("Disable signing while armed?", isPresented: $armedWarning) {
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("Vehicle is armed. ArduPilot will refuse to disable signing while armed and PX4 will not accept the disable packet without a valid signature. The disable attempt will likely time out and leave the link in an inconsistent state.\n\nDisarm the vehicle first.")
        }
        .alert("Delete signing key", isPresented: shown($confirmDelete), presenting: confirmDelete) { name in
            Button("OK", role: .destructive) {
                Task {
                    _ = await offMain { Qgc.invoke(SIGNING_REMOVE, name) }
                    revision += 1
                }
            }
            Button("Cancel", role: .cancel) {}
        } message: { name in
            Text("Are you sure you want to delete '\(name)'?\n\nIf a vehicle still has this key configured, you will no longer be able to communicate with it over a signed connection. Raw or generated keys cannot be recovered — Export the hex first if you may need it later.")
        }
        .alert(exported.map { "Export key: \($0)" } ?? "", isPresented: shown($exported)) {
            Button("OK", role: .cancel) {}
        } message: {
            Text("Key copied to clipboard. Store it securely — it will be cleared from the clipboard in 30 seconds.")
        }
        .sheet(isPresented: $adding, onDismiss: { revision += 1 }) {
            AddKeyDialog(minPassphrase: read?.minPassphraseLength ?? 8) { adding = false }
        }
    }

    private func section(_ keys: SigningKeys) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Signing keys should only be sent to the vehicle over secure links (e.g. USB).")
                .font(.bodySmall)
                .foregroundStyle(theme.colors.onSurfaceVariant)
            if keys.vehicle {
                HStack {
                    Text("Active key").font(.bodyLarge).frame(maxWidth: .infinity, alignment: .leading)
                    Text(keys.activeKey).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                }
            }
            ForEach(keys.keys, id: \.name) { key in
                keyRow(keys, key)
            }
            if let refused {
                Text(refused).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
            if keys.keys.isEmpty {
                Text("No keys configured").font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            Button("Add key") { adding = true }.buttonStyle(.bordered)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, Space.s4)
        .padding(.vertical, Space.s2)
    }

    private func keyRow(_ keys: SigningKeys, _ key: SigningKeyRow) -> some View {
        let buttons = keyButtons(keys, key)
        return HStack(spacing: Space.s1) {
            Text(key.name).font(.bodyLarge)
            if buttons.otherActive {
                Text(" (another key active)").font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            Spacer(minLength: 0)
            if buttons.enable {
                Button(buttons.pending ? "Configuring…" : "Enable") { confirmEnable = key.name }
                    .disabled(!keys.vehicle || buttons.pending)
            }
            if buttons.disable {
                Button(buttons.pending ? "Disabling…" : "Disable") {
                    if keys.armed { armedWarning = true } else { change(SIGNING_DISABLE) }
                }
                .disabled(buttons.pending)
            }
            if !key.inUse {
                Button("Export") { export(key.name) }
                Button("Delete") { confirmDelete = key.name }
            }
        }
        .buttonStyle(.borderless)
    }

    private func change(_ path: String, _ args: Any?...) {
        Task {
            refused = await offMain { refusalOf(path, args) }
            revision += 1
        }
    }

    private func export(_ name: String) {
        Task {
            let hex = await offMain { textOf(Qgc.invokeResult(SIGNING_EXPORT, name)) }
            if let hex, !hex.isEmpty {
                copyForAWhile(hex)
                exported = name
            }
        }
    }
}

private struct AddKeyDialog: View {
    let minPassphrase: Int
    let onDone: () -> Void
    @State private var name = ""
    @State private var raw = false
    @State private var passphrase = ""
    @State private var hex = ""
    @State private var error: String?
    @State private var saving = false
    @Environment(\.theme) private var theme

    var body: some View {
        let secret = raw ? hex : passphrase
        NavigationStack {
            Form {
                Section("Key name") {
                    TextField("Enter a friendly name", text: $name)
                }
                Section {
                    Picker("Key type", selection: $raw) {
                        Text("Passphrase").tag(false)
                        Text("Raw key (hex)").tag(true)
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    if !raw {
                        SecureField("Enter passphrase (min \(minPassphrase) chars)", text: $passphrase)
                            .autocorrectionDisabled()
                            .textInputAutocapitalization(.never)
                        if !passphrase.isEmpty && passphrase.count < minPassphrase {
                            Text("Passphrase too short (\(passphrase.count)/\(minPassphrase))").font(.bodySmall).foregroundStyle(theme.colors.error)
                        }
                    } else {
                        HStack {
                            TextField("64 hex characters", text: Binding(
                                get: { hex },
                                set: { typed in hex = String(typed.filter(\.isHexDigit).prefix(RAW_KEY_HEX_LENGTH)) }
                            ))
                            .autocorrectionDisabled()
                            .textInputAutocapitalization(.never)
                            .font(.system(.body, design: .monospaced))
                            Button("Generate") {
                                Task { hex = await offMain { textOf(Qgc.invokeResult(SIGNING_GENERATE)) } ?? "" }
                            }
                            .buttonStyle(.borderless)
                        }
                        if !hex.isEmpty && hex.count != RAW_KEY_HEX_LENGTH {
                            Text("\(hex.count)/64 hex characters").font(.bodySmall).foregroundStyle(theme.colors.error)
                        }
                    }
                    if let error {
                        Text(error).font(.bodySmall).foregroundStyle(theme.colors.error)
                    }
                }
            }
            .navigationTitle("Add signing key")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", action: onDone).disabled(saving)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(saving ? "Adding…" : "OK") { add(secret) }
                        .disabled(saving || !canAddKey(name, raw, secret, minPassphrase))
                }
            }
        }
        .interactiveDismissDisabled(saving)
    }

    private func add(_ secret: String) {
        saving = true
        let path = raw ? SIGNING_ADD_RAW : SIGNING_ADD_PASSPHRASE
        let keyName = name
        Task {
            let refusal = await offMain { refusalOf(path, [keyName, secret]) }
            saving = false
            if refusal == nil { onDone() } else { error = refusal }
        }
    }
}
