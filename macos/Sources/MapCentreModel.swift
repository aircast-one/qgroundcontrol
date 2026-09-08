import Foundation

struct MapCentreState: Equatable {
    var missionPoints: [GeoPoint] = []
    var otherPoints: [GeoPoint] = []
    var launch: GeoPoint?
    var vehicle: GeoPoint?
    var gcs: GeoPoint?

    var allPoints: [GeoPoint] { missionPoints + otherPoints }
}

enum MapCentre: String, CaseIterable, Identifiable {
    case mission
    case allItems
    case launch
    case vehicle
    case myLocation
    case coordinates

    var id: String { rawValue }

    var title: String {
        switch self {
        case .mission: return "Mission"
        case .allItems: return "Everything"
        case .launch: return "Launch"
        case .vehicle: return "Vehicle"
        case .myLocation: return "My Location"
        case .coordinates: return "Coordinates\u{2026}"
        }
    }

    func enabled(in state: MapCentreState) -> Bool {
        switch self {
        case .mission: return !state.missionPoints.isEmpty
        case .allItems: return !state.allPoints.isEmpty
        case .launch: return state.launch != nil
        case .vehicle: return state.vehicle != nil
        case .myLocation: return state.gcs != nil
        case .coordinates: return true
        }
    }

    func frame(in state: MapCentreState) -> MapFrame? {
        let points: [GeoPoint]
        switch self {
        case .mission: points = state.missionPoints
        case .allItems: points = state.allPoints
        case .launch: points = state.launch.map { [$0] } ?? []
        case .vehicle: points = state.vehicle.map { [$0] } ?? []
        case .myLocation: points = state.gcs.map { [$0] } ?? []
        case .coordinates: return nil
        }
        guard !points.isEmpty else { return nil }
        return MapFrame(latitudes: points.map(\.latitude), longitudes: points.map(\.longitude))
    }

    static func usable(_ coordinate: [String: Any]?) -> GeoPoint? {
        guard let coordinate,
              (coordinate["valid"] as? NSNumber)?.boolValue ?? true,
              let point = GeoPoint(json: coordinate),
              point.latitude != 0 || point.longitude != 0 else { return nil }
        return point
    }

    static func frame(latitude: Double, longitude: Double) -> MapFrame? {
        guard let point = GeoPoint(json: ["latitude": latitude as NSNumber,
                                          "longitude": longitude as NSNumber]),
              abs(point.latitude) <= 90, abs(point.longitude) <= 180 else { return nil }
        return MapFrame(latitudes: [point.latitude], longitudes: [point.longitude])
    }
}
