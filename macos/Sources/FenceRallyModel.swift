import Foundation

struct GeoPoint: Equatable {
    let latitude: Double
    let longitude: Double

    init(latitude: Double, longitude: Double) {
        self.latitude = latitude
        self.longitude = longitude
    }

    init?(json: Any?) {
        guard let object = json as? [String: Any],
              let latitude = (object["latitude"] as? NSNumber)?.doubleValue,
              let longitude = (object["longitude"] as? NSNumber)?.doubleValue,
              latitude.isFinite, longitude.isFinite else { return nil }
        self.latitude = latitude
        self.longitude = longitude
    }
}

struct MapWindow: Equatable {
    let topLeft: GeoPoint
    let bottomRight: GeoPoint

    init?(centre: GeoPoint?, latitudeSpan: Double?, longitudeSpan: Double?) {
        guard let centre, let latitudeSpan, let longitudeSpan,
              latitudeSpan > 0, longitudeSpan > 0 else { return nil }
        topLeft = GeoPoint(latitude: centre.latitude + latitudeSpan / 2,
                           longitude: centre.longitude - longitudeSpan / 2)
        bottomRight = GeoPoint(latitude: centre.latitude - latitudeSpan / 2,
                               longitude: centre.longitude + longitudeSpan / 2)
    }
}

struct FenceShape: Identifiable {
    enum Form {
        case polygon(vertices: Int, area: Double)
        case circle(radius: Double)
    }

    let id: Int
    let inclusion: Bool
    let form: Form
    let latitude: Double?
    let longitude: Double?
    let vertices: [GeoPoint]
    let radiusUnits: String

    init(json: [String: Any], id: Int, circle: Bool) {
        self.id = id
        inclusion = (json["inclusion"] as? NSNumber)?.boolValue ?? true

        let center = json["center"] as? [String: Any]
        latitude = (center?["latitude"] as? NSNumber)?.doubleValue
        longitude = (center?["longitude"] as? NSNumber)?.doubleValue
        vertices = circle ? [] : ((json["path"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))

        radiusUnits = (((json["facts"] as? [[String: Any]]) ?? [])
            .first { ($0["name"] as? String) == "Radius" }?["units"] as? String) ?? "m"

        form = circle
            ? .circle(radius: ((json["facts"] as? [[String: Any]]) ?? [])
                .first { ($0["name"] as? String) == "Radius" }
                .flatMap { ($0["value"] as? NSNumber)?.doubleValue } ?? 0)
            : .polygon(vertices: (json["count"] as? NSNumber)?.intValue ?? 0,
                       area: (json["area"] as? NSNumber)?.doubleValue ?? 0)
    }

    var kindText: String {
        switch form {
        case .circle: return inclusion ? "Keep-in circle" : "Keep-out circle"
        case .polygon: return inclusion ? "Keep-in polygon" : "Keep-out polygon"
        }
    }

    var detailText: String {
        switch form {
        case let .circle(radius):
            return String(format: "%.0f %@ radius", radius, radiusUnits)
        case let .polygon(vertices, area):
            let vertexText = "\(vertices) vertice\(vertices == 1 ? "" : "s")"
            return area > 0
                ? "\(vertexText) · \(FenceShape.areaText(area))"
                : vertexText
        }
    }

    var radius: Double? {
        switch form {
        case let .circle(radius): return radius
        case .polygon: return nil
        }
    }

    var centre: GeoPoint? {
        guard let latitude, let longitude else { return nil }
        return GeoPoint(latitude: latitude, longitude: longitude)
    }

    var framingPoints: [GeoPoint] {
        guard let radius, let centre else { return vertices }
        let metresPerDegree = 111_320.0
        let latitudeSpan = radius / metresPerDegree
        let longitudeSpan = radius / (metresPerDegree * max(cos(centre.latitude * .pi / 180), 0.01))
        return [
            GeoPoint(latitude: centre.latitude - latitudeSpan, longitude: centre.longitude - longitudeSpan),
            GeoPoint(latitude: centre.latitude + latitudeSpan, longitude: centre.longitude + longitudeSpan),
        ]
    }

    var rowDetail: String {
        switch form {
        case .circle: return ""
        case .polygon: return detailText
        }
    }

    var shapeText: String {
        switch form {
        case .circle: return "Circle"
        case .polygon: return "Polygon"
        }
    }

    var usable: Bool {
        switch form {
        case .circle: return centre != nil
        case .polygon: return vertices.count >= 3
        }
    }

    var centreText: String {
        guard let latitude, let longitude else { return "—" }
        return String(format: "%.6f, %.6f", latitude, longitude)
    }

    static func areaText(_ area: Double) -> String {
        area >= 10000
            ? String(format: "%.2f km²", area / 1_000_000)
            : String(format: "%.0f m²", area)
    }
}

struct RallyPointRow: Identifiable {
    static let altitudeFact = "RelativeAltitude"

    let id: Int
    let latitude: Double?
    let longitude: Double?
    let altitude: Double?
    let altitudeUnits: String
    let altitudeIndex: Int?

    init(json: [String: Any], id: Int) {
        self.id = id
        let coordinate = json["coordinate"] as? [String: Any]
        latitude = (coordinate?["latitude"] as? NSNumber)?.doubleValue
        longitude = (coordinate?["longitude"] as? NSNumber)?.doubleValue

        let fields = (json["textFieldFacts"] as? [[String: Any]]) ?? []
        let found = fields.firstIndex { ($0["name"] as? String) == RallyPointRow.altitudeFact }
        altitudeIndex = found
        let fact = found.map { fields[$0] }
        altitude = (fact?["value"] as? NSNumber)?.doubleValue
            ?? (coordinate?["altitude"] as? NSNumber)?.doubleValue
        altitudeUnits = (fact?["units"] as? String) ?? "m"
    }

    var positionText: String {
        guard let latitude, let longitude else { return "—" }
        return String(format: "%.6f, %.6f", latitude, longitude)
    }

    var altitudeText: String {
        guard let altitude, altitude.isFinite else { return "—" }
        return String(format: "%.1f %@", altitude, altitudeUnits)
    }
}
