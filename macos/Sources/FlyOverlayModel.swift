import Foundation

struct FlyOverlays: Equatable {
    var orbitCentre: GeoPoint?
    var orbitRadius: Double = 0
    var orbitActive = false
    var roiActive = false
    var goingTo: GeoPoint?

    static let none = FlyOverlays()

    var showsOrbit: Bool { orbitActive && orbitCentre != nil && orbitRadius > 0 }

    var showsGoto: Bool { goingTo != nil }

    var roiNote: String {
        roiActive
            ? "The camera is locked on a spot. The vehicle does not say where, so the map cannot show it."
            : ""
    }

    var summary: String {
        let parts = [showsOrbit ? "orbit" : nil,
                     roiActive ? "look-at" : nil,
                     showsGoto ? "fly-to" : nil].compactMap { $0 }
        return parts.isEmpty ? "nothing in progress" : parts.joined(separator: ", ")
    }

    static func read(orbitCircle: [String: Any]?, radius: Double,
                     orbitActive: Bool, roiActive: Bool) -> FlyOverlays {
        var built = FlyOverlays()
        built.orbitActive = orbitActive
        built.roiActive = roiActive
        built.orbitCentre = GeoPoint(json: orbitCircle?["center"])
        built.orbitRadius = radius.isFinite && radius > 0 ? radius : 0
        return built
    }

    static func keepsGoto(flightMode: String, gotoFlightMode: String) -> Bool {
        !flightMode.isEmpty && flightMode == gotoFlightMode
    }
}
