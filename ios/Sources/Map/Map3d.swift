import SwiftUI

private let MAP3D_STORE = "fly-map-3d"
private let SHOWN_KEY = "shown"
private let HINTED_KEY = "hinted"
private let map3dStore = UserDefaults(suiteName: MAP3D_STORE) ?? .standard

@MainActor
@Observable
final class Map3d {
    static let shared = Map3d()
    private(set) var shown = map3dStore.bool(forKey: SHOWN_KEY)

    func show(_ on: Bool) {
        shown = on
        map3dStore.set(on, forKey: SHOWN_KEY)
    }

    var hinted: Bool { map3dStore.bool(forKey: HINTED_KEY) }

    func markHinted() {
        map3dStore.set(true, forKey: HINTED_KEY)
    }
}
