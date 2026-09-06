import Foundation

struct FactSection: Identifiable {
    let title: String
    let facts: [Fact]
    var id: String { title }
    var showsUnits: Bool { facts.contains { !$0.units.isEmpty } }
}

final class SettingsStore: ObservableObject {
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
