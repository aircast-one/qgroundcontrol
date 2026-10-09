import SwiftUI
import UniformTypeIdentifiers

let OFFLINE_EXPORT = "offlineMaps.export"
let OFFLINE_IMPORT = "offlineMaps.import"
private let TILESET_EXTENSION = "qgctiledb"
private let EXPORT_STAGE = "tile-export.\(TILESET_EXTENSION)"
private let IMPORT_STAGE = "tile-import.\(TILESET_EXTENSION)"
private let EXPORT_NAME = "export.\(TILESET_EXTENSION)"

private enum Transfer: String {
    case Exporting, Importing

    var label: String { rawValue }
}

func exportArguments(_ path: String, _ chosen: Set<Int64>) -> [Any] {
    [path] + chosen.sorted().map { $0 as Any }
}

private func staging(_ name: String) -> URL {
    URL.cachesDirectory.appendingPathComponent(name)
}

private func importedRefusal(_ source: URL, _ replace: Bool) -> String? {
    let staged = staging(IMPORT_STAGE)
    let scoped = source.startAccessingSecurityScopedResource()
    defer {
        if scoped { source.stopAccessingSecurityScopedResource() }
        try? FileManager.default.removeItem(at: staged)
    }
    try? FileManager.default.removeItem(at: staged)
    let copied = (try? FileManager.default.copyItem(at: source, to: staged)) != nil
    return copied ? refusal(Qgc.call(OFFLINE_IMPORT, staged.path, replace)) : "That file could not be read."
}

struct TileSetTransfer: View {
    let sets: [OfflineSet]
    let onRefusal: (String?) -> Void
    @State private var running: Transfer?
    @State private var choosingSets = false
    @State private var choosingMode = false
    @State private var chosen: Set<Int64> = []
    @State private var replace = false
    @State private var importing = false
    @State private var exported: URL?

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            ActionLine(label: "Import map tiles", button: "Import…", enabled: running == nil) { choosingMode = true }
            .fileImporter(isPresented: $importing, allowedContentTypes: [.item]) { result in
                guard case .success(let source) = result else { return }
                running = .Importing
                let replacing = replace
                Task { finish(await offMain { importedRefusal(source, replacing) }) }
            }
            ActionLine(label: "Export map tiles", button: "Export…", enabled: running == nil) {
                chosen = []
                choosingSets = true
            }
            .fileMover(
                isPresented: Binding(get: { exported != nil }, set: { if !$0 { exported = nil } }),
                file: exported,
                onCompletion: { result in
                    try? FileManager.default.removeItem(at: staging(EXPORT_NAME))
                    switch result {
                    case .success: finish(nil)
                    case .failure: finish("That file could not be written.")
                    }
                },
                onCancellation: {
                    try? FileManager.default.removeItem(at: staging(EXPORT_NAME))
                    running = nil
                }
            )
            if let running {
                Text(running.label)
                ProgressView().frame(maxWidth: .infinity)
            }
        }
        .sheet(isPresented: $choosingSets) { exportChooser }
        .confirmationDialog("Import tile sets", isPresented: $choosingMode, titleVisibility: .visible) {
            Button("Append to existing sets") { pickImport(false) }
            Button("Replace existing sets") { pickImport(true) }
            Button("Cancel", role: .cancel) {}
        }
    }

    private var exportChooser: some View {
        NavigationStack {
            List(sets, id: \.id) { set in
                Toggle(set.name, isOn: Binding(
                    get: { chosen.contains(set.id) },
                    set: { chosen = $0 ? chosen.union([set.id]) : chosen.subtracting([set.id]) }
                ))
            }
            .navigationTitle("Export selected tile sets")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("Cancel") { choosingSets = false } }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Ok") {
                        choosingSets = false
                        startExport()
                    }
                    .disabled(chosen.isEmpty)
                }
            }
        }
        .presentationDetents([.medium, .large])
    }

    private func pickImport(_ mode: Bool) {
        replace = mode
        importing = true
    }

    private func startExport() {
        running = .Exporting
        let staged = staging(EXPORT_STAGE)
        let named = staging(EXPORT_NAME)
        let arguments = exportArguments(staged.path, chosen)
        Task {
            let refused = await offMain { () -> String? in
                let files = FileManager.default
                try? files.removeItem(at: staged)
                try? files.removeItem(at: named)
                let refused = refusal(Qgc.call(OFFLINE_EXPORT, arguments: arguments))
                    ?? ((try? files.moveItem(at: staged, to: named)) == nil ? "That file could not be written." : nil)
                try? files.removeItem(at: staged)
                return refused
            }
            if let refused {
                finish(refused)
            } else {
                exported = named
            }
        }
    }

    private func finish(_ refusal: String?) {
        running = nil
        onRefusal(refusal)
    }
}

private struct ActionLine: View {
    let label: String
    let button: String
    let enabled: Bool
    let onClick: () -> Void

    var body: some View {
        HStack {
            Text(label).frame(maxWidth: .infinity, alignment: .leading)
            Button(button, action: onClick)
                .buttonStyle(.bordered)
                .disabled(!enabled)
        }
    }
}
