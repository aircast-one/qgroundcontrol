import Foundation

struct VehicleTrack: Equatable {
    let available: Bool
    let recording: Bool
    let vehicleId: Int?
    let generation: Int
    let dropped: Int
    let count: Int
    let points: [GeoPoint]

    static let none = VehicleTrack()

    private init() {
        available = false
        recording = false
        vehicleId = nil
        generation = 0
        dropped = 0
        count = 0
        points = []
    }

    init(_ json: [String: Any]) {
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        recording = (json["recording"] as? NSNumber)?.boolValue ?? false
        vehicleId = (json["vehicleId"] as? NSNumber)?.intValue
        generation = (json["generation"] as? NSNumber)?.intValue ?? 0
        dropped = (json["dropped"] as? NSNumber)?.intValue ?? 0
        count = (json["count"] as? NSNumber)?.intValue ?? 0
        points = ((json["points"] as? [Any]) ?? []).compactMap { GeoPoint(json: $0) }
    }

    var draws: Bool { available && points.count > 1 }

    // The count is the core's, not a cap repeated here: the head has no business knowing how
    // many positions the trail holds, only that the start of the flight is no longer on it.
    var notice: String {
        guard dropped > 0 else { return "" }
        return "Trail trimmed — showing the last \(count) positions of this flight."
    }
}
