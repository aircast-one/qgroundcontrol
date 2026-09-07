import Foundation

// Which page a window is showing is state a test must be able to reach: with the screen
// locked no window can become key, so a synthesised click on the sidebar does nothing.
// Keeping it in @State would make every multi-page window untestable for that reason.
final class PageSelection: ObservableObject, Probeable {
    static var probeID: String { "pages" }

    @Published var page: String

    private let owner: String
    private let pages: [String]

    init(owner: String, pages: [String]) {
        self.owner = owner
        self.pages = pages
        page = pages.first ?? ""
    }

    var identifier: String { "\(owner).pages" }

    func probeState() -> [String: Any] {
        ["owner": owner, "page": page, "pages": pages]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "select" else {
            return ["ok": false, "error": "unknown action \(action)"]
        }
        guard let requested = args["page"], pages.contains(requested) else {
            return ["ok": false, "error": "no page \(args["page"] ?? "")", "pages": pages]
        }
        page = requested
        return ["ok": true, "state": probeState()]
    }
}
