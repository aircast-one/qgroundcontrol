import Foundation

struct FlyOverlays: Equatable {
    var orbitCentre: GeoPoint?
    var orbitRadius: Double = 0
    var orbitActive = false
    var roiActive = false
    var roiAt: GeoPoint?
    var goingTo: GeoPoint?
    var clickedAt: GeoPoint?

    static let none = FlyOverlays()

    var showsOrbit: Bool { orbitActive && orbitCentre != nil && orbitRadius > 0 }

    var showsGoto: Bool { goingTo != nil }

    var showsRoi: Bool { roiActive && roiAt != nil }

    var roiNote: String {
        guard roiActive else { return "" }
        return roiAt == nil
            ? "The camera is locked on a spot the vehicle has not given a position for."
            : "The camera is locked on the marked spot."
    }

    var summary: String {
        let parts = [showsOrbit ? "orbit" : nil,
                     roiActive ? "look-at" : nil,
                     showsGoto ? "fly-to" : nil].compactMap { $0 }
        return parts.isEmpty ? "nothing in progress" : parts.joined(separator: ", ")
    }

    static func read(orbitCircle: [String: Any]?, radius: Double,
                     orbiting: Bool?, roiActive: Bool,
                     roi: [String: Any]? = nil) -> FlyOverlays {
        var built = FlyOverlays()
        built.orbitActive = orbiting == true
        built.roiActive = roiActive
        built.roiAt = roiActive ? MapCentre.usable(roi) : nil
        built.orbitCentre = GeoPoint(json: orbitCircle?["center"])
        built.orbitRadius = radius.isFinite && radius > 0 ? radius : 0
        return built
    }

    static func keepsGoto(flightMode: String, gotoFlightMode: String) -> Bool {
        !flightMode.isEmpty && flightMode == gotoFlightMode
    }
}
