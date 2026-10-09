import SwiftUI
import UniformTypeIdentifiers

let SCRIPTING_VIEW = "view.scripting"
let SCRIPTING_SCREEN = "scripting"
let SCRIPTING_OPEN = "scripting.open"
let SCRIPTING_UPLOAD = "scripting.upload"
let SCRIPTING_DOWNLOAD = "scripting.download"
let SCRIPTING_DELETE = "scripting.delete"
let SCRIPTING_CANCEL = "scripting.cancel"
private let BUSY_POLL_MS = 300
private let IDLE_POLL_MS = 2000

struct Scripting: Equatable {
    let available: Bool
    let enabled: Bool
    let enable: Fact?
    let scripts: [String]
    let busy: Bool
    let progress: Float
    let status: String
    let unsupportedText: String
}

struct ScriptRefusal: Equatable {
    let title: String
    let text: String
}

func scriptRefusal(_ title: String, _ reason: String?, _ fallback: String) -> ScriptRefusal? {
    reason.map { ScriptRefusal(title: title, text: $0.ifBlank(fallback)) }
}

func scripting(_ view: JSON?) -> Scripting? {
    guard let it = view else { return nil }
    return Scripting(
        available: it["available"].bool,
        enabled: it["enabled"].bool,
        enable: it["enable"].object != nil ? factFromControl(it["enable"]) : nil,
        scripts: it["scripts"].array.map(\.string),
        busy: it["busy"].bool,
        progress: Float(it["progress"].double(0)),
        status: it["status"].string,
        unsupportedText: it["unsupportedText"].string
    )
}

private func copyPickedScript(_ from: URL, _ target: URL) -> Bool {
    let scoped = from.startAccessingSecurityScopedResource()
    defer { if scoped { from.stopAccessingSecurityScopedResource() } }
    try? FileManager.default.removeItem(at: target)
    return (try? FileManager.default.copyItem(at: from, to: target)) != nil
}

private let scriptDownloads = FileManager.default.temporaryDirectory.appending(path: "scripting-download")
private let UNSAFE_SCRIPT_NAMES: Set<String> = ["", ".", ".."]

func scriptFileName(_ name: String) -> String? {
    let flat = name.replacingOccurrences(of: "/", with: "_")
    return UNSAFE_SCRIPT_NAMES.contains(flat) ? nil : flat
}

struct ScriptingScreen: View {
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var read: Scripting?
    @State private var shownRefusal: ScriptRefusal?
    @State private var confirmDelete: String?
    @State private var choosingUpload = false
    @State private var downloaded: URL?

    var body: some View {
        VStack(spacing: 0) { content }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            .task {
                _ = await offMain { Qgc.invoke(SCRIPTING_OPEN) }
                revision += 1
            }
            .task(id: revision) {
                read = await offMain { scripting(Qgc.get(SCRIPTING_VIEW)) }
                guard (try? await Task.sleep(for: .milliseconds(read?.busy == true ? BUSY_POLL_MS : IDLE_POLL_MS))) != nil else { return }
                revision += 1
            }
            .fileImporter(isPresented: $choosingUpload, allowedContentTypes: [.item]) { result in
                switch result {
                case .success(let chosen): upload(chosen)
                case .failure(let error): shownRefusal = ScriptRefusal(title: "Lua Upload", text: error.localizedDescription)
                }
            }
            .fileMover(
                isPresented: Binding(get: { downloaded != nil }, set: { if !$0 { downloaded = nil } }),
                file: downloaded,
                onCompletion: { result in
                    if case .failure(let error) = result {
                        shownRefusal = ScriptRefusal(title: "Lua Download", text: error.localizedDescription)
                        discardDownload()
                    }
                    downloaded = nil
                },
                onCancellation: {
                    discardDownload()
                    downloaded = nil
                }
            )
            .alert("Delete Lua script", isPresented: Binding(get: { confirmDelete != nil }, set: { if !$0 { confirmDelete = nil } }), presenting: confirmDelete) { name in
                Button("OK") {
                    confirmDelete = nil
                    remove(name)
                }
                Button("Cancel", role: .cancel) { confirmDelete = nil }
            } message: { name in
                Text("Are you sure you want to delete the script \"\(name)\"? This action cannot be undone.")
            }
            .alertWhenPresenterFree($shownRefusal, title: \.title) { _ in
                Button("OK", role: .cancel) { shownRefusal = nil }
            } message: { shown in
                Text(shown.text)
            }
    }

    @ViewBuilder private var content: some View {
        if let page = read {
            if !page.available {
                Text(page.unsupportedText).padding(16).frame(maxWidth: .infinity, alignment: .leading)
            } else {
                ScrollView { form(page) }
            }
        }
    }

    private func form(_ page: Scripting) -> some View {
        let usable = page.enabled && !page.busy
        return VStack(alignment: .leading, spacing: 4) {
            if let enable = page.enable {
                FactRow(fact: enable, title: "Enable scripting", onWrite: { revision += 1 })
            }
            VStack(alignment: .leading, spacing: 6) {
                if !page.status.isBlank { Text(page.status).font(.bodyMedium) }
                Button("Upload") { choosingUpload = true }
                    .buttonStyle(.bordered)
                    .disabled(!usable)
                ForEach(page.scripts, id: \.self) { name in
                    HStack {
                        Text(name).font(.bodyLarge).frame(maxWidth: .infinity, alignment: .leading)
                        Button("Download") { download(name) }.disabled(!usable)
                        Button("Delete") { confirmDelete = name }.disabled(!usable)
                    }
                    .buttonStyle(.borderless)
                }
                if page.busy {
                    HStack(spacing: 12) {
                        Button("Cancel operation") { cancel() }.buttonStyle(.borderless)
                        Text("Transferring... \(Int((page.progress * 100).rounded()))%")
                    }
                    ProgressView(value: page.progress)
                }
            }
            .padding(.horizontal, 20)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func discardDownload() {
        downloaded.map { try? FileManager.default.removeItem(at: $0) }
    }

    private func cancel() {
        Task {
            _ = await offMain { Qgc.refusalOf(SCRIPTING_CANCEL) }
            revision += 1
        }
    }

    private func upload(_ chosen: URL) {
        Task {
            shownRefusal = await offMain {
                guard let name = scriptFileName(chosen.lastPathComponent) else { return ScriptRefusal(title: "Lua Upload", text: "Upload failed") }
                let staged = FileManager.default.temporaryDirectory.appending(path: "upload-\(name)")
                let copied = copyPickedScript(chosen, staged)
                return scriptRefusal("Lua Upload", copied ? Qgc.refusalOf(SCRIPTING_UPLOAD, staged.path, name) : "File \(name) does not exist", "Upload failed")
            }
            revision += 1
        }
    }

    private func download(_ name: String) {
        guard let file = scriptFileName(name) else {
            shownRefusal = ScriptRefusal(title: "Lua Download", text: "Download failed")
            return
        }
        let staged = scriptDownloads.appending(path: file)
        Task {
            let failed = await offMain {
                try? FileManager.default.createDirectory(at: scriptDownloads, withIntermediateDirectories: true)
                try? FileManager.default.removeItem(at: staged)
                return scriptRefusal("Lua Download", Qgc.refusalOf(SCRIPTING_DOWNLOAD, name, staged.path, name), "Download failed")
            }
            shownRefusal = failed
            while failed == nil, await offMain({ scripting(Qgc.get(SCRIPTING_VIEW))?.busy }) == true {
                try? await Task.sleep(for: .milliseconds(BUSY_POLL_MS))
            }
            if failed == nil, FileManager.default.fileExists(atPath: staged.path) {
                downloaded = staged
            }
            revision += 1
        }
    }

    private func remove(_ name: String) {
        Task {
            shownRefusal = await offMain { scriptRefusal("Lua Delete", Qgc.refusalOf(SCRIPTING_DELETE, name), "Delete failed") }
            revision += 1
        }
    }
}
