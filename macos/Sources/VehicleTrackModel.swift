import Foundation

struct VehicleTrack: Equatable {
    let available: Bool
    let recording: Bool
    let vehicleId: Int?
    let generation: Int
    let count: Int
    let points: [GeoPoint]

    static let none = VehicleTrack()

    private init() {
        available = false
        recording = false
        vehicleId = nil
        generation = 0
        count = 0
        points = []
    }

    init(_ json: [String: Any]) {
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        recording = (json["recording"] as? NSNumber)?.boolValue ?? false
        vehicleId = (json["vehicleId"] as? NSNumber)?.intValue
        generation = (json["generation"] as? NSNumber)?.intValue ?? 0
        count = (json["count"] as? NSNumber)?.intValue ?? 0
        points = ((json["points"] as? [Any]) ?? []).compactMap { GeoPoint(json: $0) }
    }

    var draws: Bool { available && points.count > 1 }
}
