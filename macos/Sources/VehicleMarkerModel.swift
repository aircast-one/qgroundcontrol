import Foundation

struct VehicleMarker: Equatable {
    let latitude: Double
    let longitude: Double
    let heading: Double?

    init?(latitude: Double?, longitude: Double?, heading: Double?) {
        guard let latitude, let longitude, latitude.isFinite, longitude.isFinite else { return nil }
        self.latitude = latitude
        self.longitude = longitude
        self.heading = heading.flatMap { $0.isFinite ? $0 : nil }
    }

    // MapKit rotates a layer anticlockwise from the positive x axis; a heading runs
    // clockwise from north, so the marker points where the vehicle does only if the
    // sign is flipped.
    var rotationRadians: Double {
        guard let heading else { return 0 }
        let normalised = heading.truncatingRemainder(dividingBy: 360)
        let clockwise = normalised < 0 ? normalised + 360 : normalised
        return -clockwise * .pi / 180
    }

    var hasHeading: Bool { heading != nil }
}
