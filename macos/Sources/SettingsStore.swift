import Foundation

struct FactSection: Identifiable {
    let title: String
    let facts: [Fact]
    var id: String { title }
    var showsUnits: Bool { facts.contains { !$0.units.isEmpty } }
}

final class SettingsStore: ObservableObject, Probeable {
    static let probeID = "settings"

    @Published private(set) var sections: [FactSection] = []
    @Published private(set) var loadError: String?
    @Published var selected: SettingsPage.ID?
    @Published var search = "" { didSet { refresh() } }

    let pages = SettingsPage.all

    private var cache: [String: [Fact]] = [:]

    func load() {
        let root = Bridge.group("settings")
        guard !((root["children"] as? [String]) ?? []).isEmpty else {
            loadError = "The settings tree is empty — the bridge is not reachable."
            sections = []
            return
        }
        loadError = nil
        if selected == nil { selected = pages.first?.id }
        cache.removeAll()
        refresh()
    }

    func refresh() {
        guard loadError == nil else { return }
        sections = search.trimmingCharacters(in: .whitespaces).isEmpty ? currentPage() : matches()
    }

    private func facts(in section: SettingsSection) -> [Fact] {
        if let cached = cache[section.path] { return cached }
        let group = Bridge.group(section.path)
        let facts = ((group["facts"] as? [[String: Any]]) ?? [])
            .compactMap { Fact(json: $0, groupPath: section.path) }
        cache[section.path] = facts
        return facts
    }

    private func currentPage() -> [FactSection] {
        guard let page = pages.first(where: { $0.id == selected }) else { return [] }
        return page.sections
            .map { FactSection(title: $0.title, facts: facts(in: $0)) }
            .filter { !$0.facts.isEmpty }
    }

    // Search spans every page, so an operator who knows the setting's name never has to
    // guess which page it lives on.
    private func matches() -> [FactSection] {
        let needle = search.trimmingCharacters(in: .whitespaces).lowercased()
        return pages.flatMap { page in
            page.sections.compactMap { section -> FactSection? in
                let hits = facts(in: section).filter {
                    $0.title.lowercased().contains(needle) || $0.name.lowercased().contains(needle)
                }
                guard !hits.isEmpty else { return nil }
                return FactSection(title: "\(page.title) › \(section.title)", facts: hits)
            }
        }
    }

    // A Fact can clamp or refuse a value, so the written value is not necessarily the
    // stored one. Drop the cache and read back rather than trusting local state.
    func write(_ fact: Fact, _ value: Any) {
        Bridge.set(fact.path, value)
        cache.removeAll()
        refresh()
    }
}

extension SettingsStore {
    func probeState() -> [String: Any] {
        [
            "page": selected ?? "",
            "search": search,
            "pages": pages.map(\.id),
            "sections": sections.map { section in
                ["title": section.title,
                 "facts": section.facts.map { ["name": $0.name, "title": $0.title,
                                                "value": $0.stringValue, "units": $0.units,
                                                "readOnly": $0.readOnly] }]
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
            guard let fact = sections.flatMap(\.facts).first(where: { $0.name == name }) else {
                return ["ok": false, "error": "no fact \(name) on this page"]
            }
            write(fact, Double(value) ?? value)
        case "reload":
            load()
        default:
            return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
