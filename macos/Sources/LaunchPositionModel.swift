import Foundation

struct LaunchPosition: Equatable {
    let vehicleHasHome: Bool
    let altitude: Double?
    let units: String
    let coordinate: GeoPoint?

    static let unknown = LaunchPosition(vehicleHasHome: false, altitude: nil,
                                        units: "m", coordinate: nil)

    init(vehicleHasHome: Bool, altitude: Double?, units: String, coordinate: GeoPoint?) {
        self.vehicleHasHome = vehicleHasHome
        self.altitude = altitude
        self.units = units
        self.coordinate = coordinate
    }

    init(home: [String: Any], item: [String: Any], altitude fact: [String: Any]) {
        vehicleHasHome = (home["valid"] as? NSNumber)?.boolValue ?? false
        self.altitude = (fact["value"] as? NSNumber)?.doubleValue
        units = (fact["units"] as? String) ?? "m"
        coordinate = GeoPoint(json: item["coordinate"])
    }

    var editable: Bool { !vehicleHasHome }

    var altitudeText: String {
        Measure.reading(altitude, units)
    }

    var positionText: String {
        guard let coordinate else { return "Not set" }
        return String(format: "%.6f, %.6f", coordinate.latitude, coordinate.longitude)
    }

    var note: String {
        """
        The ground height at this point fills the altitude in. \
        The vehicle sets its real launch point when it flies.
        """
    }
}
