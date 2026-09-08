import Foundation

enum MissionItemKind: String, CaseIterable, Identifiable {
    case waypoint
    case takeoff
    case land
    case roi
    case survey
    case corridor
    case structure

    var id: String { rawValue }

    var title: String {
        switch self {
        case .waypoint: return "Waypoint"
        case .takeoff: return "Takeoff"
        case .land: return "Land"
        case .roi: return "Region of Interest"
        case .survey: return "Survey"
        case .corridor: return "Corridor Scan"
        case .structure: return "Structure Scan"
        }
    }

    var symbol: String {
        switch self {
        case .waypoint: return "mappin"
        case .takeoff: return "arrow.up.right"
        case .land: return "arrow.down.right"
        case .roi: return "eye"
        case .survey: return "square.grid.3x3"
        case .corridor: return "road.lanes"
        case .structure: return "building.2"
        }
    }

    var invokable: String {
        switch self {
        case .waypoint: return "insertSimpleMissionItem"
        case .takeoff: return "insertTakeoffItem"
        case .land: return "insertLandItem"
        case .roi: return "insertROIMissionItem"
        case .survey, .corridor, .structure: return "insertComplexMissionItem"
        }
    }

    var complexName: String? {
        switch self {
        case .survey: return "Survey"
        case .corridor: return "Corridor Scan"
        case .structure: return "Structure Scan"
        default: return nil
        }
    }

    enum Geometry: Equatable {
        case none
        case area(String)
        case line(String)
    }

    var geometry: Geometry {
        switch self {
        case .survey: return .area("surveyAreaPolygon")
        case .structure: return .area("structurePolygon")
        case .corridor: return .line("corridorPolyline")
        default: return .none
        }
    }

    var placementHint: String {
        switch self {
        case .roi: return "Click the map to place a region of interest."
        case .survey: return "Click the map to place a survey area."
        case .corridor: return "Click the map to place a corridor to scan along."
        case .structure: return "Click the map to place a structure to scan around."
        default: return "Click the map to place a \(title.lowercased())."
        }
    }

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

    static func defaultLine(latitude: Double, longitude: Double) -> [GeoPoint] {
        let metresPerDegree = 111_320.0
        let span = defaultAreaMetres / metresPerDegree
        return [
            GeoPoint(latitude: latitude - span, longitude: longitude),
            GeoPoint(latitude: latitude + span, longitude: longitude),
        ]
    }

    static func areaProperty(forCommand command: String) -> String? {
        guard let kind = allCases.first(where: { $0.complexName == command }) else { return nil }
        if case .area(let property) = kind.geometry { return property }
        return nil
    }

    static func lineProperty(forCommand command: String) -> String? {
        guard let kind = allCases.first(where: { $0.complexName == command }) else { return nil }
        if case .line(let property) = kind.geometry { return property }
        return nil
    }

    static func seed(for kind: MissionItemKind, latitude: Double, longitude: Double) -> (property: String, points: [GeoPoint])? {
        switch kind.geometry {
        case .none:
            return nil
        case .area(let property):
            return (property, defaultArea(latitude: latitude, longitude: longitude))
        case .line(let property):
            return (property, defaultLine(latitude: latitude, longitude: longitude))
        }
    }
}
