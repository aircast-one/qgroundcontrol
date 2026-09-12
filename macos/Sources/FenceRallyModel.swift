import Foundation

struct GeoPoint: Equatable {
    let latitude: Double
    let longitude: Double

    static let places = 6

    static func text(_ latitude: Double?, _ longitude: Double?) -> String {
        guard let latitude, let longitude, latitude.isFinite, longitude.isFinite else {
            return "\u{2014}"
        }
        return String(format: "%.\(places)f, %.\(places)f", latitude, longitude)
    }

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

struct FirmwareFence: Equatable {
    let radiusMetres: Double
    let radiusText: String
    let centre: GeoPoint?

    static let title = "Firmware fence"
    static let unplacedDetail = "The vehicle enforces this, and has not reported where from yet"
    static let detail = "The vehicle enforces this from its own parameters"

    var boundary: String { FenceShape.enforced }

    var drawable: Bool { centre != nil }

    var rowDetail: String { drawable ? FirmwareFence.detail : FirmwareFence.unplacedDetail }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let metres = (json["radiusMetres"] as? NSNumber)?.doubleValue,
              metres.isFinite, metres > 0 else { return nil }
        radiusMetres = metres
        radiusText = (json["radiusText"] as? String) ?? ""
        centre = GeoPoint(json: json["centre"])
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

    static let keepIn = "keepIn"
    static let keepOut = "keepOut"
    static let enforced = "enforced"

    var boundary: String { inclusion ? FenceShape.keepIn : FenceShape.keepOut }

    var rowDetail: String { isCircle ? "" : detailText }

    var framingPoints: [GeoPoint] { framing }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let index = (json["index"] as? NSNumber)?.intValue,
              let shape = json["shape"] as? String else { return nil }
        self.index = index
        self.shape = shape
        path = (json["path"] as? String) ?? ""
        inclusion = (json["inclusion"] as? NSNumber)?.boolValue ?? false
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
    let altitudeText: String

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
        altitudeText = (json["altitudeText"] as? String) ?? Measure.unreported
    }

    init(coordinate: [String: Any]?) {
        id = -1
        path = ""
        latitude = (coordinate?["latitude"] as? NSNumber)?.doubleValue
        longitude = (coordinate?["longitude"] as? NSNumber)?.doubleValue
        altitude = nil
        altitudeUnits = ""
        altitudePath = ""
        altitudeText = Measure.unreported
    }

    static func list(_ json: Any?) -> [RallyPointRow] {
        ((json as? [Any]) ?? []).compactMap(RallyPointRow.init)
    }

    var positionText: String { GeoPoint.text(latitude, longitude) }
}

// What an empty Fence or Rally tab tells the operator. It lived in the view as a three-way
// condition, where nothing could reach it: a sentence claiming a vehicle's firmware lacks a
// feature is a claim about the vehicle, and getting it wrong stops an operator trying something
// that would have worked.
enum PlanShapeAbsence {
    static let noFence = "No geofence in this plan."
    static let noRally = "No rally points in this plan."

    static func fence(connected: Bool, supported: Bool) -> String {
        guard connected else { return noFence }
        return supported
            ? "No geofence. Nothing will stop the vehicle leaving the area."
            : "This vehicle's firmware does not support geofences."
    }

    static func rally(connected: Bool, supported: Bool) -> String {
        guard connected else { return noRally }
        return supported
            ? "No rally points. On a failsafe the vehicle returns to launch."
            : "This vehicle's firmware does not support rally points."
    }
}
