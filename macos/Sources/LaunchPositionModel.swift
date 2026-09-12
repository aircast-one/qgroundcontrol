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

    init(home: [String: Any], item: [String: Any]) {
        self.init(vehicleHasHome: (home["valid"] as? NSNumber)?.boolValue ?? false,
                  altitude: (item["altitude"] as? NSNumber)?.doubleValue,
                  units: (item["altitudeUnits"] as? String) ?? Measure.defaultUnits,
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
