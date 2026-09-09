import Foundation

struct SetupPageInfo: Equatable {
    let name: String
    let native: Bool
    let parameterSections: Bool

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let name = json["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        native = (json["native"] as? NSNumber)?.boolValue ?? false
        parameterSections = (json["parameterSections"] as? NSNumber)?.boolValue ?? false
    }
}

struct SetupGroup: Identifiable, Equatable {
    let title: String
    let pages: [SetupPageInfo]

    var id: String { title }

    init(title: String, pages: [SetupPageInfo]) {
        self.title = title
        self.pages = pages
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let title = json["title"] as? String, !title.isEmpty else { return nil }
        self.title = title
        pages = ((json["pages"] as? [Any]) ?? []).compactMap(SetupPageInfo.init)
    }
}

enum SetupCatalogue {
    static func groups(_ json: Any?) -> [SetupGroup] {
        ((json as? [Any]) ?? []).compactMap(SetupGroup.init)
    }

    // The core decides which pages this head can draw, and that answer changes with the firmware:
    // a page it does not claim would open on the summary, which reads as the window losing its way.
    static func offered(_ groups: [SetupGroup]) -> [SetupGroup] {
        groups.map { SetupGroup(title: $0.title, pages: $0.pages.filter(\.native)) }
            .filter { !$0.pages.isEmpty }
    }

    static func names(_ groups: [SetupGroup]) -> [String] {
        groups.flatMap { $0.pages.map(\.name) }
    }

    static func page(_ name: String, in groups: [SetupGroup]) -> SetupPageInfo? {
        groups.lazy.compactMap { $0.pages.first { $0.name == name } }.first
    }
}
