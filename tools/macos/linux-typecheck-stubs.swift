// Stand-ins for the few UI-side names that non-UI files reach into, so tools/macos/linux-typecheck.sh
// can typecheck every file that imports no UI framework on a Linux toolchain. Typecheck only:
// nothing here is compiled into the app or into the pure-logic checks.

import Foundation

struct MissionMap {
    static var lastRender: [String: [String: Any]] = [:]
    static var lastScale: [String: MapScaleBar] = [:]
    static var rendererCalls = 0
    static var rendererKinds: Set<String> = []
}
final class VehicleSetupWindow {
    static let shared = VehicleSetupWindow()
    func show() {}
}

// Mission.swift is typechecked with its MapKit and CoreLocation imports stripped (see
// linux-typecheck.sh); these are the map-tile probe and location-permission names it uses.
struct MKTileOverlayPath {
    var x: Int
    var y: Int
    var z: Int
    var contentScaleFactor: Double
}

final class CachedTileOverlay {
    static var served = 0
    static var fromParent = 0
    static var fromChildren = 0
    static var missed = 0
    static var requested = 0
    static var lastRequest: Date?

    static func currentMapType() -> String { "" }

    init(mapType: String) {}

    func loadTile(at path: MKTileOverlayPath, result: @escaping (Data?, Error?) -> Void) {}
}

enum CLAuthorizationStatus {
    case notDetermined, restricted, denied, authorizedAlways, authorizedWhenInUse, authorized
}

final class CLLocationManager {
    var authorizationStatus: CLAuthorizationStatus { .notDetermined }
}
