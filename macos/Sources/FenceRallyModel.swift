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

struct FenceShape: Identifiable, Equatable {
    let index: Int
    let path: String
    let shape: String
    let inclusion: Bool
    let kindText: String
    let detailText: String
    let vertices: [GeoPoint]
    let framing: [GeoPoint]
    let usable: Bool
    let centre: GeoPoint?
    let centreText: String
    let radius: Double?
    let radiusUnits: String

    var id: String { path }

    var isCircle: Bool { shape == "circle" }

    var shapeText: String { isCircle ? "Circle" : "Polygon" }

    var rowDetail: String { isCircle ? "" : detailText }

    var framingPoints: [GeoPoint] { framing }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let index = (json["index"] as? NSNumber)?.intValue,
              let shape = json["shape"] as? String else { return nil }
        self.index = index
        self.shape = shape
        path = (json["path"] as? String) ?? ""
        inclusion = (json["inclusion"] as? NSNumber)?.boolValue ?? true
        kindText = (json["kindText"] as? String) ?? ""
        detailText = (json["detailText"] as? String) ?? ""
        vertices = ((json["vertices"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
        framing = ((json["framing"] as? [Any]) ?? []).compactMap(GeoPoint.init(json:))
        usable = (json["usable"] as? NSNumber)?.boolValue ?? false
        centre = GeoPoint(json: json["centre"])
        centreText = (json["centreText"] as? String) ?? "\u{2014}"
        radius = (json["radius"] as? NSNumber)?.doubleValue
        radiusUnits = (json["radiusUnits"] as? String) ?? "m"
    }

    static func list(_ json: Any?) -> [FenceShape] {
        ((json as? [Any]) ?? []).compactMap(FenceShape.init)
    }
}

struct RallyPointRow: Identifiable, Equatable {
    let id: Int
    let path: String
    let latitude: Double?
    let longitude: Double?
    let altitude: Double?
    let altitudeUnits: String
    let altitudePath: String

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let index = (json["index"] as? NSNumber)?.intValue else { return nil }
        id = index
        path = (json["path"] as? String) ?? ""
        latitude = (json["latitude"] as? NSNumber)?.doubleValue
        longitude = (json["longitude"] as? NSNumber)?.doubleValue
        altitude = (json["altitude"] as? NSNumber)?.doubleValue
        altitudeUnits = (json["altitudeUnits"] as? String) ?? ""
        altitudePath = (json["altitudePath"] as? String) ?? ""
    }

    init(coordinate: [String: Any]?) {
        id = -1
        path = ""
        latitude = (coordinate?["latitude"] as? NSNumber)?.doubleValue
        longitude = (coordinate?["longitude"] as? NSNumber)?.doubleValue
        altitude = nil
        altitudeUnits = ""
        altitudePath = ""
    }

    static func list(_ json: Any?) -> [RallyPointRow] {
        ((json as? [Any]) ?? []).compactMap(RallyPointRow.init)
    }

    var positionText: String {
        guard let latitude, let longitude else { return "\u{2014}" }
        return String(format: "%.6f, %.6f", latitude, longitude)
    }

    var altitudeText: String {
        Measure.reading(altitude, altitudeUnits)
    }
}
