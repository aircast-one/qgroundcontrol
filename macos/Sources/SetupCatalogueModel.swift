import Foundation

struct SetupPageInfo: Equatable {
    let name: String
    let parameterSections: Bool
    let openable: Bool
    let blockedReason: String?

    init(name: String, parameterSections: Bool,
         openable: Bool = true, blockedReason: String? = nil) {
        self.name = name
        self.parameterSections = parameterSections
        self.openable = openable
        self.blockedReason = blockedReason
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let name = json["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        parameterSections = (json["parameterSections"] as? NSNumber)?.boolValue ?? false
        openable = (json["openable"] as? NSNumber)?.boolValue ?? true
        blockedReason = (json["blockedReason"] as? String).flatMap { $0.isEmpty ? nil : $0 }
    }

    // The core joins a page to its component on the KnownVehicleComponent enum where a firmware
    // declares one and on the C++ class name otherwise - five pages by enum, eight by class, one
    // deliberately unbacked - so all fourteen are decided without a translated string anywhere.
    var blockedSentence: String? {
        guard !openable else { return nil }
        guard let blockedReason else { return VehicleComponentInfo.blockedWithoutReason }
        return "Disabled while the vehicle is \(blockedReason)"
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

    // Which pages this head can draw is this head's own knowledge. view.setup used to answer it
    // with a native flag, which made the core say what a head is capable of, and the sidebar went
    // blank the moment that flag moved. The window draws a bespoke view for the names it knows and
    // parameter sections for anything else that has them, so that is the question asked here.
    static func offered(_ groups: [SetupGroup]) -> [SetupGroup] {
        groups.map { SetupGroup(title: $0.title, pages: $0.pages.filter(SetupPage.draws)) }
            .filter { !$0.pages.isEmpty }
    }

    static func names(_ groups: [SetupGroup]) -> [String] {
        groups.flatMap { $0.pages.map(\.name) }
    }

    static func page(_ name: String, in groups: [SetupGroup]) -> SetupPageInfo? {
        groups.lazy.compactMap { $0.pages.first { $0.name == name } }.first
    }
}
