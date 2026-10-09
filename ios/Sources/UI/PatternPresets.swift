import SwiftUI

let SAVE_PRESET = "core.plan.savePreset"
let APPLY_PRESET = "core.plan.applyPreset"
let DELETE_PRESET = "core.plan.deletePreset"

func presetKind(_ view: JSON?) -> String? {
    view.flatMap { $0["presetKind"].stringOrNil }.flatMap { $0.isBlank || $0 == "null" ? nil : $0 }
}

func presetNames(_ view: JSON?) -> [String] { view?["names"].strings ?? [] }

struct PatternPresets: View {
    let index: Int
    let kind: String
    let onApplied: () -> Void
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var names: [String] = []
    @State private var chosen: String?
    @State private var saving = false
    @State private var deleting: String?
    @State private var refusal: String?
    @State private var typed = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Presets").font(.titleSmall)
            HStack(spacing: Space.s2) {
                Text("Preset")
                Menu {
                    ForEach(names, id: \.self) { name in
                        Button(name) { chosen = name }
                    }
                } label: {
                    Text(chosen ?? "None")
                }
                .buttonStyle(.bordered)
                .disabled(names.isEmpty)
            }
            HStack(spacing: Space.s2) {
                Button("Apply preset") { if let chosen { act(APPLY_PRESET, [index, chosen], then: onApplied) } }
                    .disabled(chosen == nil)
                Button("Delete preset") { deleting = chosen }
                    .disabled(chosen == nil)
            }
            .buttonStyle(.text)
            Button("Save settings as new preset") {
                typed = ""
                saving = true
            }
            .buttonStyle(.text)
            if let refusal { Text(refusal).foregroundStyle(theme.colors.error) }
        }
        .padding(.horizontal, Space.s5)
        .padding(.vertical, Space.s2)
        .task(id: [kind, String(revision)]) {
            let kind = kind
            names = await offMain { presetNames(Qgc.get("view.patternPresets(\(kind))")) }
            chosen = chosen.flatMap { names.contains($0) ? $0 : nil } ?? names.first
        }
        .alert("Delete preset", isPresented: Binding(get: { deleting != nil }, set: { if !$0 { deleting = nil } }), presenting: deleting) { name in
            Button("OK") {
                deleting = nil
                act(DELETE_PRESET, [kind, name])
            }
            Button("Cancel", role: .cancel) { deleting = nil }
        } message: { name in
            Text("Are you sure you want to delete '\(name)' preset?")
        }
        .background {
            if saving {
                PlanDialog(title: "Save preset", onDismiss: { saving = false }) {
                    VStack(alignment: .leading, spacing: Space.s2) {
                        Text("Save the current settings as a named preset.")
                        Text("Preset name").font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                        TextField("Enter preset name", text: $typed)
                            .textFieldStyle(.roundedBorder)
                        if let error = presetNameError(typed) {
                            Text(error).font(.bodySmall).foregroundStyle(theme.colors.error)
                        }
                    }
                } buttons: {
                    Button("Cancel") { saving = false }
                    Button("Save") {
                        saving = false
                        let name = typed.trimmed
                        act(SAVE_PRESET, [index, name]) { chosen = name }
                    }
                    .disabled(presetNameError(typed) != nil)
                }
            }
        }
    }

    private func act(_ path: String, _ args: [Any], then: @escaping () -> Void = {}) {
        Task {
            refusal = await offMain { Qgc.refusalOf(path, arguments: args) }
            revision += 1
            if refusal == nil { then() }
        }
    }
}

func presetNameError(_ typed: String) -> String? {
    typed.trimmed.isEmpty ? "Preset name cannot be blank."
        : typed.contains("/") ? "Preset name cannot include the \"/\" character."
        : nil
}
