import Foundation

// Which page a window is showing is state a test must be able to reach: with the screen
// locked no window can become key, so a synthesised click on the sidebar does nothing.
// Keeping it in @State would make every multi-page window untestable for that reason.
final class PageSelection: ObservableObject, Probeable {
    static var probeID: String { "pages" }

    @Published var page: String
    @Published private(set) var pages: [String]

    private let owner: String

    init(owner: String, pages: [String]) {
        self.owner = owner
        self.pages = pages
        page = pages.first ?? ""
    }

    // The vehicle setup pages come from the core and change with the firmware. An empty answer is
    // a read that failed rather than a window with no pages, so it is ignored.
    func offer(_ listed: [String]) {
        guard !listed.isEmpty, listed != pages else { return }
        pages = listed
        if !listed.contains(page) { page = listed.first ?? page }
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
