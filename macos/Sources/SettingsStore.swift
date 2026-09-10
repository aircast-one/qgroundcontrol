import Foundation

final class SettingsStore: ObservableObject, Probeable, WriteReporting {
    @Published var writeFailure: String?
    static let probeID = "settings"

    @Published private(set) var pages: [SettingsPage] = []
    @Published private(set) var sections: [SettingsSection] = []
    @Published private(set) var loadError: String?
    @Published var selected: SettingsPage.ID?
    @Published var search = "" { didSet { refresh() } }

    private var cache: [String: [SettingsSection]] = [:]

    func load() {
        let read = SettingsPage.list(Bridge.group("view.settings")["pages"])
        guard !read.isEmpty else {
            loadError = "The settings tree is empty — the bridge is not reachable."
            pages = []
            sections = []
            return
        }
        loadError = nil
        pages = read
        if selected == nil || !read.contains(where: { $0.id == selected }) {
            selected = read.first?.id
        }
        cache.removeAll()
        refresh()
    }

    func refresh() {
        guard loadError == nil else { return }
        let read = search.trimmingCharacters(in: .whitespaces).isEmpty ? currentPage() : matches()
        if read != sections { sections = read }
    }

    private func sections(of page: String) -> [SettingsSection] {
        if let cached = cache[page] { return cached }
        let read = SettingsSection.list(Bridge.group("view.settings(\(page))")["sections"])
            .filter { !$0.controls.isEmpty }
        cache[page] = read
        return read
    }

    private func currentPage() -> [SettingsSection] {
        guard let selected else { return [] }
        return sections(of: selected)
    }

    // Search spans every page, so an operator who knows the setting's name never has to
    // guess which page it lives on.
    private func matches() -> [SettingsSection] {
        let needle = search.trimmingCharacters(in: .whitespaces).lowercased()
        return pages.flatMap { page in
            sections(of: page.title).compactMap { section -> SettingsSection? in
                let hits = section.controls.filter {
                    $0.label.lowercased().contains(needle) || $0.name.lowercased().contains(needle)
                }
                guard !hits.isEmpty else { return nil }
                return SettingsSection(title: "\(page.title) › \(section.title)",
                                       group: section.group, path: section.path, note: "",
                                       subsections: [SettingsSubsection(title: "", controls: hits)])
            }
        }
    }

    // A Fact can clamp or refuse a value, so the written value is not necessarily the
    // stored one. Drop the cache and read back rather than trusting local state.
    func write(_ control: SettingsControl, _ value: Any) {
        guard !control.readOnly else {
            writeFailure = FactWrite.readOnly
            return
        }
        write(control.path, value, control.label)
        cache.removeAll()
        refresh()
    }
}

extension SettingsStore {
    func probeState() -> [String: Any] {
        [
            "writeFailure": writeFailure ?? "",
            "page": selected ?? "",
            "search": search,
            "pages": pages.map(\.id),
            "sections": sections.map { section in
                ["title": section.title,
                 "facts": section.controls.map { ["name": $0.name, "title": $0.label,
                                                  "value": $0.valueString, "units": $0.units,
                                                  "readOnly": $0.readOnly,
                                                  "detail": $0.rowDescription(label: $0.label),
                                                  "control": String(describing: $0.kind)] }]
            },
        ]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "select":
            guard let page = args["page"], pages.contains(where: { $0.id == page }) else {
                return ["ok": false, "error": "no page \(args["page"] ?? "")", "pages": pages.map(\.id)]
            }
            selected = page
            refresh()
        case "search":
            search = args["text"] ?? ""
        case "set":
            guard let name = args["name"], let value = args["value"] else {
                return ["ok": false, "error": "set needs name and value"]
            }
            guard let control = sections.flatMap(\.controls).first(where: { $0.name == name }) else {
                return ["ok": false, "error": "no fact \(name) on this page"]
            }
            write(control, Double(value) ?? value)
        case "reload":
            load()
        default:
            return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
