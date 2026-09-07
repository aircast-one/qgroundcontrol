import Foundation

enum MissionItemKind: String, CaseIterable, Identifiable {
    case waypoint
    case takeoff
    case land
    case roi
    case survey

    var id: String { rawValue }

    var title: String {
        switch self {
        case .waypoint: return "Waypoint"
        case .takeoff: return "Takeoff"
        case .land: return "Land"
        case .roi: return "Region of Interest"
        case .survey: return "Survey"
        }
    }

    var symbol: String {
        switch self {
        case .waypoint: return "mappin"
        case .takeoff: return "arrow.up.right"
        case .land: return "arrow.down.right"
        case .roi: return "eye"
        case .survey: return "square.grid.3x3"
        }
    }

    var invokable: String {
        switch self {
        case .waypoint: return "insertSimpleMissionItem"
        case .takeoff: return "insertTakeoffItem"
        case .land: return "insertLandItem"
        case .roi: return "insertROIMissionItem"
        case .survey: return "insertComplexMissionItem"
        }
    }

    var complexName: String? { self == .survey ? "Survey" : nil }

    var placementHint: String {
        switch self {
        case .roi: return "Click the map to place a region of interest."
        case .survey: return "Click the map to place a survey area."
        default: return "Click the map to place a \(title.lowercased())."
        }
    }

    // QGC gives a new survey a default area to work from rather than an empty one, so
    // the item is complete the moment it is placed.
    static let defaultAreaMetres = 150.0

    static func defaultArea(latitude: Double, longitude: Double) -> [GeoPoint] {
        let metresPerDegree = 111_320.0
        let latitudeSpan = defaultAreaMetres / metresPerDegree
        let longitudeSpan = defaultAreaMetres / (metresPerDegree * max(cos(latitude * .pi / 180), 0.01))
        return [
            GeoPoint(latitude: latitude - latitudeSpan, longitude: longitude - longitudeSpan),
            GeoPoint(latitude: latitude - latitudeSpan, longitude: longitude + longitudeSpan),
            GeoPoint(latitude: latitude + latitudeSpan, longitude: longitude + longitudeSpan),
            GeoPoint(latitude: latitude + latitudeSpan, longitude: longitude - longitudeSpan),
        ]
    }
}
