import SwiftUI
import UniformTypeIdentifiers

private let OPEN_FOLDER = "plan-open"
private let OPEN_FALLBACK = "opened.plan"
private let SAVE_CACHE = "saving.plan"
private let KML_CACHE = "export.kml"
private let MISSION_ROOT = "plan.missionController"
private let PLAN_STATUS_VIEW = "view.plan"
private let MISSION_ITEMS_VIEW = "view.missionItems"

let PLAN_OPEN_TYPES: [UTType] = [.item]
let PLAN_FILE_TYPE = UTType(filenameExtension: PLAN_EXTENSION, conformingTo: .json) ?? .json
let KML_FILE_TYPE = UTType(filenameExtension: KML_EXTENSION, conformingTo: .xml) ?? .xml

struct PatternChoice {
    let options: () -> [String]
    let pick: (String) -> Void
    let cancel: () -> Void
}

func planLoad(_ path: String) -> Bool? {
    if case .bool(let loaded) = Qgc.invokeResult("\(PLAN_ROOT).loadFromFile", path) { return loaded }
    return nil
}

func loadFailureMessage(_ loaded: Bool?) -> String? {
    loaded == nil ? "The plan could not be loaded. The current plan is unchanged." : nil
}

func planSave(_ path: String) -> String? {
    guard Qgc.invokeResult("\(PLAN_ROOT).saveToFile", path) == .bool(true) else { return nil }
    return currentPlanPath(Qgc.get(PLAN_STATUS_VIEW))
}

private func copyIn(_ source: URL, _ into: URL) -> Bool {
    let scoped = source.startAccessingSecurityScopedResource()
    defer { if scoped { source.stopAccessingSecurityScopedResource() } }
    guard let data = try? Data(contentsOf: source) else { return false }
    return (try? data.write(to: into)) != nil && !data.isEmpty
}

private func copyOut(_ from: URL, _ target: URL) -> Bool {
    let scoped = target.startAccessingSecurityScopedResource()
    defer { if scoped { target.stopAccessingSecurityScopedResource() } }
    guard let data = try? Data(contentsOf: from) else { return false }
    return (try? data.write(to: target)) != nil
}

func patternNames(_ view: JSON?) -> [String] {
    (view?["patterns"].arrayOrNil ?? []).filter { $0.object != nil }.map { $0["name"].string }.filter { !$0.isBlank }
}

private func patternNames() -> [String] { patternNames(Qgc.get(PLAN_STATUS_VIEW)) }

func currentPlanPath(_ view: JSON?) -> String? {
    view.flatMap { $0["filePath"].string.isBlank ? nil : $0["filePath"].string }
}

func visualItems() -> [JSON] { Qgc.get(MISSION_ITEMS_VIEW)["items"].array }

private func lastDistance(_ items: [JSON]) -> Double? {
    guard let last = items.last, case .number(let distance) = last["patternDistance"] else { return nil }
    return distance
}

private let TEXT_MISSION_SUFFIXES: Set<String> = ["waypoints", "txt"]

func openedFileName(_ shown: String?) -> String {
    shown.map { $0.split(separator: "/", omittingEmptySubsequences: false).last.map(String.init) ?? $0 }
        .flatMap { $0.isBlank || $0 == "." || $0 == ".." ? nil : $0 } ?? OPEN_FALLBACK
}

func isTextMission(_ name: String) -> Bool {
    name.lastIndex(of: ".").map { TEXT_MISSION_SUFFIXES.contains(String(name[name.index(after: $0)...])) } ?? false
}

private func displayName(_ uri: URL) -> String { uri.lastPathComponent }

private let FILES_STORE = "plan-files"
private let LAST_DOCUMENT_KEY = "lastDocument"

private func lastDocument() -> URL? {
    UserDefaults(suiteName: FILES_STORE)?.string(forKey: LAST_DOCUMENT_KEY).flatMap(URL.init(string:))
}

private func rememberDocument(_ uri: URL) {
    UserDefaults(suiteName: FILES_STORE)?.set(uri.absoluteString, forKey: LAST_DOCUMENT_KEY)
}

private let DOWNLOAD_POLL_MS = 200
private let DOWNLOAD_POLLS = 300

private func downloadSettled(_ left: Int = DOWNLOAD_POLLS) async -> Bool {
    guard left > 0 else { return false }
    guard await offMain({ planIsSyncing(Qgc.get(PLAN_STATUS_VIEW)) }) else { return true }
    try? await Task.sleep(for: .milliseconds(DOWNLOAD_POLL_MS))
    return await downloadSettled(left - 1)
}

private var cacheDir: URL { FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0] }

private func stagedPlan() -> Data? {
    let staged = cacheDir.appending(path: SAVE_CACHE)
    try? FileManager.default.removeItem(at: staged)
    return planSave(staged.path).flatMap { try? Data(contentsOf: URL(fileURLWithPath: $0)) }
}

private func stagedKml() -> Data? {
    let staged = cacheDir.appending(path: KML_CACHE)
    try? FileManager.default.removeItem(at: staged)
    Qgc.invoke("\(PLAN_ROOT).saveToKml", staged.path)
    return (try? Data(contentsOf: staged)).flatMap { $0.isEmpty ? nil : $0 }
}

private func writePlan(_ target: URL) -> String? {
    let staged = cacheDir.appending(path: SAVE_CACHE)
    try? FileManager.default.removeItem(at: staged)
    guard let written = planSave(staged.path) else { return "The plan could not be saved." }
    return copyOut(URL(fileURLWithPath: written), target) ? nil : "The plan was written but could not be copied out."
}

private func loadStaged(_ chosen: URL) -> (String?, Bool) {
    let staged = cacheDir.appending(path: OPEN_FOLDER).appending(path: openedFileName(displayName(chosen)))
    let folder = staged.deletingLastPathComponent()
    try? FileManager.default.removeItem(at: folder)
    try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
    guard copyIn(chosen, staged) else { return ("That file could not be read.", false) }
    let loaded = planLoad(staged.path)
    return (loadFailureMessage(loaded), loaded == true)
}

private func importBoundaryFiles(_ uris: [URL], _ pattern: String) -> String? {
    let labelled = uris.map { ($0, displayName($0)) }
    let main = mainBoundaryName(labelled.map(\.1))
    let label = labelled.first { $0.1 == main }?.1
    let staged = cacheDir.appending(path: boundaryCacheName(label))
    let stale = (try? FileManager.default.contentsOfDirectory(at: cacheDir, includingPropertiesForKeys: nil)) ?? []
    stale.filter { $0.lastPathComponent.hasPrefix("boundary.") }.forEach { try? FileManager.default.removeItem(at: $0) }
    guard labelled.allSatisfy({ uri, name in copyIn(uri, cacheDir.appending(path: boundaryCacheName(name))) }) else {
        return "That file could not be read."
    }
    let before = visualItems().count
    Qgc.invoke("\(MISSION_ROOT).insertComplexMissionItemFromKMLOrSHP", pattern, staged.path, before, true)
    let after = visualItems()
    if after.count <= before { return "\(label ?? "That file") added nothing to the plan." }
    if importedNothing(lastDistance(after)) {
        Qgc.invoke("\(MISSION_ROOT).removeVisualItem", after.count - 1)
        return "\(label ?? "That file") holds no area for a \(pattern)."
    }
    return nil
}

struct PlanFileDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.data] }
    static var writableContentTypes: [UTType] { [PLAN_FILE_TYPE, KML_FILE_TYPE, .data] }

    let data: Data

    init(data: Data) { self.data = data }

    init(configuration: ReadConfiguration) throws {
        data = configuration.file.regularFileContents ?? Data()
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: data)
    }
}

enum PlanPicker { case open, importBoundary }

struct PlanExport {
    let document: PlanFileDocument
    let name: String
    let kml: Bool
}

@MainActor
@Observable
final class PlanFileActions {
    fileprivate var picking = false
    fileprivate var picker = PlanPicker.open
    fileprivate var exportShown = false
    fileprivate var exporting: PlanExport?
    private var openedCount = 0
    private var pendingImport: [URL]?
    private var patterns: [String] = []
    fileprivate var openPlan: OpenPlanDocument?
    @ObservationIgnored fileprivate var onResult: (String) -> Void = { _ in }

    func open() {
        picker = .open
        picking = true
    }

    func saveAs() { guarded { self.export(self.openPlan?.name ?? DEFAULT_PLAN_NAME) } }

    func save() {
        guarded {
            if let target = self.openPlan?.document { self.writeTo(target) } else { self.export(DEFAULT_PLAN_NAME) }
        }
    }

    func exportKml() {
        guarded {
            Task {
                guard let data = await offMain({ stagedKml() }) else { return self.onResult("The plan could not be exported.") }
                self.exporting = PlanExport(document: PlanFileDocument(data: data), name: DEFAULT_KML_NAME, kml: true)
                self.exportShown = true
            }
        }
    }

    func importBoundary() {
        picker = .importBoundary
        picking = true
    }

    var patternChoice: PatternChoice {
        PatternChoice(
            options: { self.pendingImport == nil ? [] : self.patterns },
            pick: { name in
                let uris = self.pendingImport
                self.pendingImport = nil
                if let uris { self.importFrom(uris, name) }
            },
            cancel: { self.pendingImport = nil }
        )
    }

    func newPlan() { discard("removeAll", "New plan.", "The plan could not be cleared.") }

    func clearMission() {
        discard("removeAllFromVehicle", "Clear sent to the vehicle.", "The clear could not be sent to the vehicle.")
    }

    func download() {
        discard("loadFromVehicle", "Loading the plan from the vehicle.", "The plan could not be loaded from the vehicle.") {
            if await downloadSettled() { self.openedCount += 1 }
        }
    }

    func documentName() -> String? { openPlan?.name }

    func opened() -> Int { openedCount }

    fileprivate func bind(_ openPlan: OpenPlanDocument, _ onResult: @escaping (String) -> Void) {
        self.openPlan = openPlan
        self.onResult = onResult
    }

    private func adopt(_ uri: URL) {
        openPlan?.document = uri
        rememberDocument(uri)
        openPlan?.name = displayName(uri)
    }

    private func forget() {
        openPlan?.document = nil
        openPlan?.name = nil
    }

    private func discard(_ method: String, _ success: String, _ failure: String, then: @escaping @MainActor () async -> Void = {}) {
        Task {
            if await offMain({ Qgc.invoke("\(PLAN_ROOT).\(method)") }) {
                forget()
                onResult(success)
                await then()
            } else {
                onResult(failure)
            }
        }
    }

    private func importFrom(_ uris: [URL], _ pattern: String) {
        Task {
            let message = await offMain { importBoundaryFiles(uris, pattern) }
            onResult(message ?? "Boundary imported.")
        }
    }

    private func guarded(_ action: @escaping () -> Void) {
        Task {
            let view = await offMain { freshPlanView() }
            if let blocked = saveBlockedReason(view) {
                PlanFocus.notReady(view)
                onResult(blocked)
            } else {
                action()
            }
        }
    }

    private func writeTo(_ target: URL) {
        Task {
            if let message = await offMain({ writePlan(target) }) {
                onResult(message)
            } else {
                adopt(target)
                onResult("Plan saved.")
            }
        }
    }

    private func export(_ name: String) {
        Task {
            guard let data = await offMain({ stagedPlan() }) else { return onResult("The plan could not be saved.") }
            exporting = PlanExport(document: PlanFileDocument(data: data), name: withExtension(name, PLAN_EXTENSION), kml: false)
            exportShown = true
        }
    }

    fileprivate func picked(_ result: Result<[URL], Error>) {
        guard case .success(let uris) = result, !uris.isEmpty else { return }
        switch picker {
        case .open: openFrom(uris[0])
        case .importBoundary: chooseBoundary(uris)
        }
    }

    private func openFrom(_ chosen: URL) {
        Task {
            let (failure, loaded) = await offMain { loadStaged(chosen) }
            guard loaded else {
                if let failure { onResult(failure) }
                return
            }
            if isTextMission(openedFileName(displayName(chosen))) { forget() } else { adopt(chosen) }
            openedCount += 1
            onResult("Plan opened.")
        }
    }

    private func chooseBoundary(_ chosen: [URL]) {
        Task {
            let names = await offMain { patternNames() }
            if names.isEmpty {
                onResult("This vehicle offers no pattern to import into.")
            } else if names.count == 1 {
                importFrom(chosen, names[0])
            } else {
                patterns = names
                pendingImport = chosen
            }
        }
    }

    fileprivate func exported(_ result: Result<URL, Error>, _ export: PlanExport) {
        switch result {
        case .success(let target):
            if export.kml {
                onResult("KML exported.")
            } else {
                adopt(target)
                onResult("Plan saved.")
            }
        case .failure(let error):
            if (error as? CocoaError)?.code == .userCancelled { return }
            onResult(export.kml ? "The KML was written but could not be copied out." : "The plan was written but could not be copied out.")
        }
    }
}

struct PlanFileDialogs: ViewModifier {
    let files: PlanFileActions
    let onResult: (String) -> Void
    @Environment(OpenPlanDocument.self) private var openPlan

    func body(content: Content) -> some View {
        content
            .onAppear { files.bind(openPlan, onResult) }
            .background {
                Color.clear
                    .frame(width: 0, height: 0)
                    .fileImporter(
                        isPresented: Binding(get: { files.picking }, set: { files.picking = $0 }),
                        allowedContentTypes: PLAN_OPEN_TYPES,
                        allowsMultipleSelection: files.picker == .importBoundary
                    ) { result in files.picked(result) }
                    .fileDialogDefaultDirectory(lastDocument()?.deletingLastPathComponent())
            }
            .background {
                Color.clear
                    .frame(width: 0, height: 0)
                    .fileExporter(
                        isPresented: Binding(get: { files.exportShown }, set: { files.exportShown = $0 }),
                        document: files.exporting?.document,
                        contentType: files.exporting?.kml == true ? KML_FILE_TYPE : PLAN_FILE_TYPE,
                        defaultFilename: files.exporting?.name
                    ) { result in
                        if let export = files.exporting { files.exported(result, export) }
                    }
                    .fileDialogDefaultDirectory(lastDocument()?.deletingLastPathComponent())
            }
    }
}
