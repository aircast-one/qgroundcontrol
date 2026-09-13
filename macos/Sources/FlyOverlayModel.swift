import Foundation

// The head used to take the ring's CENTRE from the raw vehicle (vehicle.orbitMapCircle.center)
// while taking its radius from view.orbit. The core already serves `centre`, built by centre_of,
// which validates both coordinates AND withholds them unless the vehicle is actually turning --
// so the raw read was a second decoder of the same value with none of the gating. It drew nothing
// wrong: showsOrbit demanded orbitActive and orbitRadius too, and those WERE gated. That is a
// coupling rather than a rule, and the next person to touch showsOrbit breaks it silently.
struct Orbit {
    let centre: GeoPoint?
    let radiusMetres: Double
    let radiusText: String
    let turning: Bool?

    init?(_ json: Any?) {
        guard let json = json as? [String: Any] else { return nil }
        guard let metres = (json["radiusMetres"] as? NSNumber)?.doubleValue,
              metres.isFinite, metres > 0 else { return nil }
        radiusMetres = metres
        radiusText = (json["radiusText"] as? String) ?? ""
        turning = (json["orbiting"] as? NSNumber)?.boolValue
        centre = GeoPoint(json: json["centre"])
    }

    var drawable: Bool { turning == true && centre != nil }
}

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

    static func read(orbit: Orbit?, roiActive: Bool,
                     roi: [String: Any]? = nil) -> FlyOverlays {
        var built = FlyOverlays()
        built.orbitActive = orbit?.turning == true
        built.roiActive = roiActive
        built.roiAt = roiActive ? MapCentre.usable(roi) : nil
        built.orbitCentre = orbit?.centre
        built.orbitRadius = orbit?.radiusMetres ?? 0
        return built
    }

    static func keepsGoto(flightMode: String, gotoFlightMode: String) -> Bool {
        !flightMode.isEmpty && flightMode == gotoFlightMode
    }
}
