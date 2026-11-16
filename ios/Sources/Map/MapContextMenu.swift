import Foundation

struct MapMenuEntry {
    let title: String
    let enabled: Bool
    let startsGroup: Bool
    let run: (() -> Void)?

    init(_ title: String, enabled: Bool = true, startsGroup: Bool = false, run: (() -> Void)? = nil) {
        self.title = title
        self.enabled = enabled
        self.startsGroup = startsGroup
        self.run = run
    }
}

typealias MapContextMenu = (MapHit?, Double, Double) -> [MapMenuEntry]?

