import Foundation

private let GEOTAG_CACHE = "geotag"

struct TreeEntry: Equatable {
    let url: URL
    let name: String
    let directory: Bool
}

private func stagingDir(_ name: String) -> URL {
    let dir = URL.cachesDirectory.appending(path: GEOTAG_CACHE).appending(path: name)
    try? FileManager.default.removeItem(at: dir)
    try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    return dir
}

func taggedOutputDir() -> URL { stagingDir("tagged") }

private func scoped<T>(_ url: URL, _ work: () -> T) -> T {
    let held = url.startAccessingSecurityScopedResource()
    defer { if held { url.stopAccessingSecurityScopedResource() } }
    return work()
}

func treeChildren(_ tree: URL) -> [TreeEntry] {
    scoped(tree) {
        ((try? FileManager.default.contentsOfDirectory(at: tree, includingPropertiesForKeys: [.isDirectoryKey])) ?? []).map { child in
            TreeEntry(url: child, name: child.lastPathComponent, directory: (try? child.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true)
        }
    }
}

private func copyInto(_ from: URL, _ target: URL) -> Bool {
    scoped(from) {
        try? FileManager.default.removeItem(at: target)
        return (try? FileManager.default.copyItem(at: from, to: target)) != nil
    }
}

func stageLog(_ uri: URL, _ name: String) -> String? {
    let target = stagingDir("log").appending(path: name.ifBlank("flight.log"))
    return copyInto(uri, target) ? target.path : nil
}

func stageImages(_ tree: URL) -> (String, Int) {
    let dir = stagingDir("images")
    let copied = scoped(tree) {
        treeChildren(tree)
            .filter { !$0.directory && isGeoTagImage($0.name) }
            .filter { copyInto($0.url, dir.appending(path: $0.name)) }
            .count
    }
    return (dir.path, copied)
}

func hasTaggedFolder(_ tree: URL) -> Bool {
    treeChildren(tree).contains { $0.name == DEFAULT_GEOTAG_OUTPUT && $0.directory }
}

func holdsImages(_ tree: URL) -> Bool {
    treeChildren(tree).contains { !$0.directory && isGeoTagImage($0.name) }
}

func treeName(_ tree: URL) -> String {
    tree.lastPathComponent.ifBlank(tree.absoluteString)
}

func documentName(_ uri: URL) -> String { uri.lastPathComponent }

func downloadedLogs(_ logSavePath: String) -> [URL] {
    let keys: [URLResourceKey] = [.isRegularFileKey, .contentModificationDateKey]
    let listed = (try? FileManager.default.contentsOfDirectory(at: URL(fileURLWithPath: logSavePath), includingPropertiesForKeys: keys)) ?? []
    let modified = { (file: URL) in (try? file.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate) ?? .distantPast }
    return listed
        .filter { (try? $0.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true && ["ulg", "bin"].contains($0.pathExtension.lowercased()) }
        .sorted { modified($0) > modified($1) }
}

private func folderIn(_ tree: URL, _ name: String) -> URL? {
    let folder = tree.appending(path: name, directoryHint: .isDirectory)
    let existing = treeChildren(tree).first { $0.name == name && $0.directory }
    return existing?.url ?? ((try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)) != nil ? folder : nil)
}

func publishTagged(_ staged: URL, _ tree: URL, _ subfolder: String?) -> Int {
    scoped(tree) {
        let folder = subfolder.flatMap { folderIn(tree, $0) } ?? tree
        let present = Set(treeChildren(folder).map(\.name))
        let files = (try? FileManager.default.contentsOfDirectory(at: staged, includingPropertiesForKeys: [.isRegularFileKey])) ?? []
        return files
            .filter { (try? $0.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true && isGeoTagImage($0.lastPathComponent) }
            .filter { file in
                let target = folder.appending(path: file.lastPathComponent)
                let created = !present.contains(file.lastPathComponent)
                let written = (try? Data(contentsOf: file).write(to: target, options: .atomic)) != nil
                if !written && created { try? FileManager.default.removeItem(at: target) }
                return written
            }
            .count
    }
}
