import Foundation

struct LaunchPosition: Equatable {
    let vehicleHasHome: Bool
    let altitude: Double?
    let units: String
    let altitudeText: String
    let coordinate: GeoPoint?

    static let unknown = LaunchPosition(vehicleHasHome: false, altitude: nil,
                                        units: Measure.defaultUnits, altitudeText: "",
                                        coordinate: nil)

    init(vehicleHasHome: Bool, altitude: Double?, units: String, altitudeText: String,
         coordinate: GeoPoint?) {
        self.vehicleHasHome = vehicleHasHome
        self.altitude = altitude
        self.units = units
        self.altitudeText = altitudeText.isEmpty ? Measure.unreported : altitudeText
        self.coordinate = coordinate
    }

    // planningFor.home is the coordinate itself or null, where the Qt object carried a `valid`
    // flag beside one. Absent and invalid collapse to the same answer on purpose: the core's
    // nested serialiser drops an unusable coordinate to null, so "there is a home" is exactly
    // "a usable point came back" and there is no third state to keep.
    init(planningForHome home: Any?, item: [String: Any]) {
        self.init(vehicleHasHome: GeoPoint(json: home) != nil,
                  altitude: (item["altitude"] as? NSNumber)?.doubleValue,
                  units: (item["altitudeEditUnits"] as? String) ?? "",
                  altitudeText: (item["altitudeText"] as? String) ?? "",
                  coordinate: GeoPoint(json: item["coordinate"]))
    }

    init(home: [String: Any], item: [String: Any]) {
        self.init(vehicleHasHome: (home["valid"] as? NSNumber)?.boolValue ?? false,
                  altitude: (item["altitude"] as? NSNumber)?.doubleValue,
                  units: (item["altitudeEditUnits"] as? String) ?? "",
                  altitudeText: (item["altitudeText"] as? String) ?? "",
                  coordinate: GeoPoint(json: item["coordinate"]))
    }

    var editable: Bool { !vehicleHasHome }

    var positionText: String {
        guard let coordinate else { return "Not set" }
        return GeoPoint.text(coordinate.latitude, coordinate.longitude)
    }

    var note: String {
        """
        The ground height at this point fills the altitude in. \
        The vehicle sets its real launch point when it flies.
        """
    }
}
