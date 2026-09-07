import Foundation

enum MissionItemKind: String, CaseIterable, Identifiable {
    case waypoint
    case takeoff
    case land
    case roi

    var id: String { rawValue }

    var title: String {
        switch self {
        case .waypoint: return "Waypoint"
        case .takeoff: return "Takeoff"
        case .land: return "Land"
        case .roi: return "Region of Interest"
        }
    }

    var symbol: String {
        switch self {
        case .waypoint: return "mappin"
        case .takeoff: return "arrow.up.right"
        case .land: return "arrow.down.right"
        case .roi: return "eye"
        }
    }

    var invokable: String {
        switch self {
        case .waypoint: return "insertSimpleMissionItem"
        case .takeoff: return "insertTakeoffItem"
        case .land: return "insertLandItem"
        case .roi: return "insertROIMissionItem"
        }
    }

    var placementHint: String {
        "Click the map to place \(self == .roi ? "a region of interest" : "a \(title.lowercased())")."
    }
}
