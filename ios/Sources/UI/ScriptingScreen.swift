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

private struct FreePresenterAlert<Item, Actions: View, Message: View>: ViewModifier {
    @Binding var item: Item?
    let title: (Item) -> String
    let actions: (Item) -> Actions
    let message: (Item) -> Message
    @State private var probe = PresenterProbe()
    @State private var ready = false

    func body(content: Content) -> some View {
        content
            .background(PresenterProbeView(probe: probe).allowsHitTesting(false))
            .task(id: item != nil) {
                ready = false
                guard item != nil else { return }
                while !Task.isCancelled, !presenterFree(probe.controller) {
                    try? await Task.sleep(for: .milliseconds(SHEET_POLL_MS))
                }
                ready = !Task.isCancelled
            }
            .alert(
                item.map(title) ?? "",
                isPresented: Binding(get: { ready && item != nil }, set: { if !$0 { item = nil } }),
                presenting: item,
                actions: actions,
                message: message
            )
    }
}

extension View {
    func alertWhenPresenterFree<Item, Actions: View, Message: View>(
        _ item: Binding<Item?>,
        title: @escaping (Item) -> String,
        @ViewBuilder actions: @escaping (Item) -> Actions,
        @ViewBuilder message: @escaping (Item) -> Message
    ) -> some View {
        modifier(FreePresenterAlert(item: item, title: title, actions: actions, message: message))
    }
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
                guard case .success(let chosen) = result else { return }
                upload(chosen)
            }
            .fileMover(
                isPresented: Binding(get: { downloaded != nil }, set: { if !$0 { downloaded = nil } }),
                file: downloaded,
                onCompletion: { _ in downloaded = nil },
                onCancellation: {
                    downloaded.map { try? FileManager.default.removeItem(at: $0) }
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

    private func cancel() {
        Task {
            _ = await offMain { Qgc.refusalOf(SCRIPTING_CANCEL) }
            revision += 1
        }
    }

    private func upload(_ chosen: URL) {
        Task {
            shownRefusal = await offMain {
                let name = chosen.lastPathComponent.replacingOccurrences(of: "/", with: "_")
                let staged = FileManager.default.temporaryDirectory.appending(path: "upload-\(name)")
                let copied = copyPickedScript(chosen, staged)
                return scriptRefusal("Lua Upload", copied ? Qgc.refusalOf(SCRIPTING_UPLOAD, staged.path, name) : "File \(name) does not exist", "Upload failed")
            }
            revision += 1
        }
    }

    private func download(_ name: String) {
        let staged = scriptDownloads.appending(path: name)
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
