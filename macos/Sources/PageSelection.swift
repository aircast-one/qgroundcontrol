import Foundation

// Which page a window is showing is state a test must be able to reach: with the screen
// locked no window can become key, so a synthesised click on the sidebar does nothing.
// Keeping it in @State would make every multi-page window untestable for that reason.
// A map click on the Plan window places whatever the CURRENT PAGE arms, and nothing otherwise.
// The banner offering to stop adding was already gated on the page; the placement was not, so
// arming a waypoint and switching to Rally without placing left the click adding a MISSION item
// while the operator was looking at the rally list -- the banner had vanished, which reads as
// cancelled.
enum PlanPlacement: Equatable {
    case mission
    case rally
    case nothing

    static let missionPage = "Mission"
    static let rallyPage = "Rally"

    static func decided(page: String, missionArmed: Bool, rallyArmed: Bool) -> PlanPlacement {
        switch page {
        case PlanPlacement.missionPage: return missionArmed ? .mission : .nothing
        case PlanPlacement.rallyPage: return rallyArmed ? .rally : .nothing
        default: return .nothing
        }
    }

    var places: Bool { self != .nothing }
}

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
