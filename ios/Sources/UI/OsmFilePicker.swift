import SwiftUI
import UniformTypeIdentifiers

private let OSM_FILE_SETTING = "settings.viewer3DSettings.osmFilePath"
private let OSM_FOLDER = "osm"
private let OSM_ENABLED_SETTING = "settings.viewer3DSettings.enabled"

func osmFileAccepted(_ name: String) -> Bool {
    let lower = name.lowercased()
    return lower.hasSuffix(".osm") || lower.hasSuffix(".xml")
}

private enum Staged {
    case Copied(path: String)
    case NotOsm
    case Unreadable
}

private func osmFolder() -> URL {
    URL.applicationSupportDirectory.appendingPathComponent(OSM_FOLDER, isDirectory: true)
}

private func stagedOsm(_ chosen: URL) -> Staged {
    let shown = chosen.lastPathComponent.replacingOccurrences(of: "[/\\\\]", with: "_", options: .regularExpression)
    guard osmFileAccepted(shown) else { return .NotOsm }
    let scoped = chosen.startAccessingSecurityScopedResource()
    defer { if scoped { chosen.stopAccessingSecurityScopedResource() } }
    let files = FileManager.default
    let folder = osmFolder()
    let incoming = folder.appendingPathComponent(".incoming")
    let staged = folder.appendingPathComponent(shown)
    do {
        try files.createDirectory(at: folder, withIntermediateDirectories: true)
        try? files.removeItem(at: incoming)
        try files.copyItem(at: chosen, to: incoming)
        try? files.removeItem(at: staged)
        try files.moveItem(at: incoming, to: staged)
        return .Copied(path: staged.path)
    } catch {
        return .Unreadable
    }
}

private func keepOnly(_ path: String) {
    let kept = URL(fileURLWithPath: path).lastPathComponent
    (try? FileManager.default.contentsOfDirectory(at: osmFolder(), includingPropertiesForKeys: nil))?
        .filter { $0.lastPathComponent != kept }
        .forEach { try? FileManager.default.removeItem(at: $0) }
}

struct OsmFilePicker: View {
    let onWrite: () -> Void
    @State private var refusal: String?
    @State private var picking = false
    @QgcBool(settingControl(OSM_ENABLED_SETTING)) private var enabled
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            Button("Choose OpenStreetMap file") { picking = true }
                .buttonStyle(.bordered)
                .disabled(!enabled)
            if let refusal {
                Text(refusal).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, Space.s4)
        .padding(.vertical, Space.s2)
        .fileImporter(isPresented: $picking, allowedContentTypes: [.item]) { result in
            switch result {
            case .success(let chosen): choose(chosen)
            case .failure(let error): refusal = error.localizedDescription
            }
        }
    }

    private func choose(_ chosen: URL) {
        Task {
            refusal = await offMain { () -> String? in
                switch stagedOsm(chosen) {
                case .Copied(let path):
                    let refused = Qgc.writeRefusal(OSM_FILE_SETTING, path)
                    if refused == nil { keepOnly(path) }
                    return refused
                case .NotOsm:
                    return "Choose an OpenStreetMap file (.osm or .xml)."
                case .Unreadable:
                    return "That file could not be read."
                }
            }
            onWrite()
        }
    }
}
