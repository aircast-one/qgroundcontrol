import Foundation

let FENCE_ROOT = "\(PLAN_ROOT).geoFenceController"
let RALLY_ROOT = "\(PLAN_ROOT).rallyPointController"

let FENCE_POLYGONS = "\(FENCE_ROOT).polygons"
let FENCE_CIRCLES = "\(FENCE_ROOT).circles"
let RALLY_POINTS = "\(RALLY_ROOT).points"
let FENCES_VIEW = "view.fences"
let FLY_FENCES_VIEW = "view.flyFences"

struct FencePolygon: Equatable {
    var index: Int
    var inclusion: Bool
    var vertices: [TrackPoint]
    var editable: EditableShape? = nil
    var kindText: String = ""
    var detailText: String = ""
}

struct FenceRow: Equatable {
    var index: Int
    var circle: Bool
    var title: String
    var detail: String
    var inclusion: Bool? = nil
    var radius: String = ""
}

func inclusionText(_ inclusion: Bool) -> String { inclusion ? "Inclusion" : "Exclusion" }

func fenceRows(_ polygons: [FencePolygon], _ circles: [FenceCircle]) -> [FenceRow] {
    polygons.map { FenceRow(index: $0.index, circle: false, title: $0.kindText.ifBlank("Polygon \($0.index + 1)"), detail: inclusionText($0.inclusion), inclusion: $0.inclusion) }
        + circles.map { circle in
            FenceRow(
                index: circle.index,
                circle: true,
                title: circle.kindText.ifBlank("Circle \(circle.index + 1)"),
                detail: inclusionText(circle.inclusion),
                radius: [trimmedRadius(circle.radius), circle.radiusUnits].filter { !$0.isBlank }.joined(separator: " ")
            )
        }
}

func fenceHeading(_ row: FenceRow, _ previous: FenceRow?) -> String? {
    previous == nil || previous?.circle != row.circle ? (row.circle ? "Circular fences" : "Polygon fences") : nil
}

func fenceRowHit(_ row: FenceRow) -> MapHit {
    row.circle ? .Circle(index: row.index) : .FenceVertex(polygon: row.index, vertex: 0)
}

func rowSelected(_ row: FenceRow, _ selected: MapHit?) -> Bool {
    switch selected {
    case .Circle(let index), .CircleCentre(let index): row.circle && index == row.index
    case .FenceVertex(let polygon, _): !row.circle && polygon == row.index
    default: false
    }
}

func fenceSelectionAfterRemove(_ row: FenceRow, _ selected: MapHit?) -> MapHit? {
    func moved(_ index: Int, _ rebuild: (Int) -> MapHit) -> MapHit? {
        index == row.index ? nil : index > row.index ? rebuild(index - 1) : selected
    }
    switch selected {
    case .FenceVertex(let polygon, let vertex):
        return row.circle ? selected : moved(polygon) { .FenceVertex(polygon: $0, vertex: vertex) }
    case .Circle(let index):
        return !row.circle ? selected : moved(index) { .Circle(index: $0) }
    case .CircleCentre(let index):
        return !row.circle ? selected : moved(index) { .CircleCentre(index: $0) }
    default:
        return selected
    }
}

func rallyAfterRemove(_ removed: Int, _ countBefore: Int) -> MapHit? {
    countBefore - 2 >= 0 ? .Rally(index: min(removed, countBefore - 2)) : nil
}

let NO_GEOFENCE = "No geofence \u{2013} keep the vehicle inside a boundary, or out of an area."
let NO_RALLY_POINTS = "No rally points \u{2013} alternate landing points for Return to Launch. Tap the map to place one."
let GEOFENCE_NOT_SUPPORTED = "Not supported \u{2013} this vehicle does not support geofence."
let RALLY_NOT_SUPPORTED = "Not supported \u{2013} this vehicle does not support rally points."

func rallyRows(_ points: [RallyPoint]) -> [FenceRow] {
    points.map { point in
        let height = point.altitude.isFinite ? "\(plainAltitude(point.altitude)) \(point.altitudeUnits.ifBlank("m"))" : nil
        let place = String(format: "%.6f, %.6f", point.latitude, point.longitude)
        return FenceRow(index: point.index, circle: false, title: "Rally \(point.index + 1)", detail: [height, place].compactMap { $0 }.joined(separator: " \u{00b7} "))
    }
}

private func plainAltitude(_ value: Double) -> String {
    value == value.rounded(.down) ? String(Int64(value)) : String(format: "%.1f", value)
}

struct FenceCircle: Equatable {
    var index: Int
    var inclusion: Bool
    var centre: TrackPoint
    var radius: Double
    var detailText: String
    var kindText: String
    var radiusMinimum: Double?
    var radiusMaximum: Double?
    var radiusMetres: Double
    var radiusUnits: String

    init(
        index: Int,
        inclusion: Bool,
        centre: TrackPoint,
        radius: Double,
        detailText: String = "",
        kindText: String = "",
        radiusMinimum: Double? = nil,
        radiusMaximum: Double? = nil,
        radiusMetres: Double? = nil,
        radiusUnits: String = ""
    ) {
        self.index = index
        self.inclusion = inclusion
        self.centre = centre
        self.radius = radius
        self.detailText = detailText
        self.kindText = kindText
        self.radiusMinimum = radiusMinimum
        self.radiusMaximum = radiusMaximum
        self.radiusMetres = radiusMetres ?? radius
        self.radiusUnits = radiusUnits
    }
}

struct RallyPoint: Equatable {
    var index: Int
    var latitude: Double
    var longitude: Double
    var altitudeMetres: Double = 0
    var altitude: Double = .nan
    var altitudeUnits: String = ""
    var altitudePath: String = ""

    static func == (lhs: RallyPoint, rhs: RallyPoint) -> Bool {
        lhs.index == rhs.index && lhs.latitude == rhs.latitude && lhs.longitude == rhs.longitude
            && lhs.altitudeMetres == rhs.altitudeMetres && (lhs.altitude == rhs.altitude || lhs.altitude.isNaN && rhs.altitude.isNaN)
            && lhs.altitudeUnits == rhs.altitudeUnits && lhs.altitudePath == rhs.altitudePath
    }
}

func coordinate(_ json: JSON?) -> TrackPoint? {
    guard let json else { return nil }
    let latitude = json["latitude"].double(.nan)
    let longitude = json["longitude"].double(.nan)
    return isPlottable(latitude, longitude) ? TrackPoint(latitude: latitude, longitude: longitude) : nil
}

private func listed(_ json: JSON?, _ key: String) -> [JSON]? { json?[key].arrayOrNil }

private func objectOrNil(_ json: JSON) -> JSON? { json.object == nil ? nil : json }

let FENCE_POLYGON_MINIMUM = 3

func cornerRemovable(_ polygon: FencePolygon?) -> Bool { polygon?.editable?.canRemoveVertex == true }

func fencePolygons(_ json: JSON?) -> [FencePolygon] {
    guard let list = listed(json, "polygons") else { return [] }
    return list.enumerated().compactMap { index, element in
        guard element.object != nil, let corners = element["vertices"].arrayOrNil else { return nil }
        let vertices = corners.compactMap { coordinate(objectOrNil($0)) }
        guard vertices.count >= FENCE_POLYGON_MINIMUM else { return nil }
        let at = element["index"].int(index)
        return FencePolygon(
            index: at,
            inclusion: element["inclusion"].bool(true),
            vertices: vertices,
            editable: editableShape("\(FENCE_POLYGONS).\(at)"),
            kindText: element["kindText"].string,
            detailText: element["detailText"].string
        )
    }
}

struct FirmwareFence: Equatable {
    var radiusMetres: Double
    var radiusText: String
    var centre: TrackPoint?
}

func firmwareFence(_ json: JSON?) -> FirmwareFence? {
    guard let served = json.flatMap({ objectOrNil($0["firmwareFence"]) }) else { return nil }
    let radius = served["radiusMetres"].double(.nan)
    guard !radius.isNaN, radius > 0 else { return nil }
    return FirmwareFence(radiusMetres: radius, radiusText: served["radiusText"].string, centre: coordinate(objectOrNil(served["centre"])))
}

private func bound(_ json: JSON, _ key: String) -> Double? {
    json[key].isNull ? nil : json[key].double.flatMap { $0.isFinite && $0 > 0 ? $0 : nil }
}

let CIRCLE_STEP = 1.5

func grownRadius(_ circle: FenceCircle) -> Double? {
    let wanted = circle.radius * CIRCLE_STEP
    guard let ceiling = circle.radiusMaximum else { return wanted }
    return circle.radius >= ceiling ? nil : min(wanted, ceiling)
}

func trimmedRadius(_ radius: Double) -> String {
    String(format: "%.1f", radius).trimmingTrailing("0").trimmingTrailing(".")
}

func typedRadius(_ text: String, _ circle: FenceCircle) -> Double? {
    typedNumber(text)
        .flatMap { $0 > 0 ? $0 : nil }
        .flatMap { wanted in circle.radiusMinimum.map { wanted >= $0 } ?? true ? wanted : nil }
        .flatMap { wanted in circle.radiusMaximum.map { wanted <= $0 } ?? true ? wanted : nil }
}

func shrunkRadius(_ circle: FenceCircle) -> Double? {
    let wanted = circle.radius / CIRCLE_STEP
    guard let floor = circle.radiusMinimum else { return wanted > 0 ? wanted : nil }
    return circle.radius <= floor ? nil : max(wanted, floor)
}

func fenceCircles(_ json: JSON?) -> [FenceCircle] {
    guard let list = listed(json, "circles") else { return [] }
    return list.enumerated().compactMap { index, element in
        guard element.object != nil, let centre = coordinate(objectOrNil(element["centre"])) else { return nil }
        let radius = element["radius"].double(.nan)
        guard !radius.isNaN, radius > 0 else { return nil }
        let metres = element["radiusMetres"].double(radius)
        return FenceCircle(
            index: element["index"].int(index),
            inclusion: element["inclusion"].bool(true),
            centre: centre,
            radius: radius,
            detailText: element["detailText"].string,
            kindText: element["kindText"].string,
            radiusMinimum: bound(element, "radiusMinimum"),
            radiusMaximum: bound(element, "radiusMaximum"),
            radiusMetres: metres.isFinite ? metres : radius,
            radiusUnits: element["radiusUnits"].string
        )
    }
}

func rallyPoints(_ json: JSON?) -> [RallyPoint] {
    guard let list = listed(json, "rallyPoints") else { return [] }
    return list.enumerated().compactMap { index, element in
        guard element.object != nil, let point = coordinate(element) else { return nil }
        let metres = element["altitudeMetres"].double(0)
        return RallyPoint(
            index: element["index"].int(index),
            latitude: point.latitude,
            longitude: point.longitude,
            altitudeMetres: metres.isFinite ? metres : 0,
            altitude: element["altitude"].double(.nan),
            altitudeUnits: element["altitudeUnits"].string,
            altitudePath: element["altitudePath"].string
        )
    }
}

func rallyMovePayload(_ latitude: Double, _ longitude: Double, _ altitudeMetres: Double) -> [String: Double] {
    coordinateJson(latitude, longitude, altitudeMetres)
}

func rallyAltitudeFor(_ rally: [RallyPoint], _ index: Int) -> Double {
    rally.first { $0.index == index }?.altitudeMetres ?? 0
}

let BREACH_RETURN_PATH = "\(FENCE_ROOT).breachReturnPoint"

struct BreachReturn: Equatable {
    var point: TrackPoint
    var altitude: Double?
    var units: String
    var altitudePath: String
}

func breachReturn(_ json: JSON?) -> BreachReturn? {
    guard let point = coordinate(json.flatMap { objectOrNil($0["breachReturnPoint"]) }) else { return nil }
    let altitude = json.flatMap { objectOrNil($0["breachReturnAltitude"]) }
    return BreachReturn(
        point: point,
        altitude: altitude.flatMap { $0["value"].isNull ? nil : $0["value"].double }.flatMap { $0.isFinite ? $0 : nil },
        units: altitude?["units"].string ?? "",
        altitudePath: altitude?["path"].string ?? ""
    )
}

enum FenceBridge {
    static func read() -> JSON? { MapBridge.read(FENCES_VIEW) }

    static func readFlown() -> JSON? { MapBridge.read(FLY_FENCES_VIEW) }

    @discardableResult static func addInclusionPolygon(_ topLeft: TrackPoint, _ bottomRight: TrackPoint) -> Bool {
        invokeOk("\(FENCE_ROOT).addInclusionPolygon", coordinateJson(topLeft.latitude, topLeft.longitude), coordinateJson(bottomRight.latitude, bottomRight.longitude))
    }

    @discardableResult static func addInclusionCircle(_ topLeft: TrackPoint, _ bottomRight: TrackPoint) -> Bool {
        invokeOk("\(FENCE_ROOT).addInclusionCircle", coordinateJson(topLeft.latitude, topLeft.longitude), coordinateJson(bottomRight.latitude, bottomRight.longitude))
    }

    @discardableResult static func addRallyPoint(_ latitude: Double, _ longitude: Double) -> Bool {
        invokeOk("\(RALLY_ROOT).addPoint", coordinateJson(latitude, longitude))
    }

    @discardableResult static func moveRallyPoint(_ index: Int, _ latitude: Double, _ longitude: Double, _ altitudeMetres: Double) -> Bool {
        setOk("\(RALLY_POINTS).\(index).coordinate", rallyMovePayload(latitude, longitude, altitudeMetres))
    }

    @discardableResult static func moveCircle(_ index: Int, _ latitude: Double, _ longitude: Double) -> Bool {
        setOk("\(FENCE_CIRCLES).\(index).center", coordinateJson(latitude, longitude))
    }

    @discardableResult static func setCircleRadius(_ index: Int, _ shown: Double) -> Bool {
        setOk("\(FENCE_CIRCLES).\(index).radius", shown)
    }

    @discardableResult static func deletePolygon(_ index: Int) -> Bool { invokeOk("\(FENCE_ROOT).deletePolygon", index) }

    @discardableResult static func setBreachReturn(_ at: TrackPoint) -> Bool {
        setOk(BREACH_RETURN_PATH, coordinateJson(at.latitude, at.longitude))
    }

    @discardableResult static func clearBreachReturn() -> Bool { setOk(BREACH_RETURN_PATH, nil) }

    @discardableResult static func setBreachAltitude(_ path: String, _ shown: Double) -> Bool { setOk(path, shown) }

    @discardableResult static func deleteCircle(_ index: Int) -> Bool { invokeOk("\(FENCE_ROOT).deleteCircle", index) }

    @discardableResult static func setRallyAltitude(_ path: String, _ shown: Double) -> Bool { setOk(path, shown) }

    @discardableResult static func removeRallyPoint(_ index: Int) -> Bool {
        invokeOk("\(RALLY_ROOT).removePoint", "@\(RALLY_POINTS).\(index)")
    }

    @discardableResult static func setPolygonInclusion(_ index: Int, _ inclusion: Bool) -> Bool {
        setOk("\(FENCE_POLYGONS).\(index).inclusion", inclusion)
    }

    @discardableResult static func removeVertex(_ polygon: Int, _ vertex: Int) -> Bool {
        invokeOk("\(FENCE_POLYGONS).\(polygon).removeVertex", vertex)
    }

    @discardableResult static func adjustVertex(_ polygon: Int, _ vertex: Int, _ latitude: Double, _ longitude: Double) -> Bool {
        invokeOk("\(FENCE_POLYGONS).\(polygon).adjustVertex", vertex, coordinateJson(latitude, longitude))
    }
}

private let EARTH_RADIUS_M = 6_371_000.0
private let CIRCLE_SEGMENTS = 48

private func radians(_ degrees: Double) -> Double { degrees * .pi / 180 }

private func degrees(_ radians: Double) -> Double { radians * 180 / .pi }

func circleRing(_ centre: TrackPoint, _ radiusMetres: Double, _ segments: Int = CIRCLE_SEGMENTS) -> [TrackPoint] {
    guard radiusMetres > 0, segments >= 3 else { return [] }
    return (0..<segments).map { step in pointAt(centre, radiusMetres, 360.0 * Double(step) / Double(segments)) }
}

func azimuthBetween(_ from: TrackPoint, _ to: TrackPoint) -> Double {
    let fromLat = radians(from.latitude)
    let toLat = radians(to.latitude)
    let deltaLon = radians(to.longitude - from.longitude)
    let y = sin(deltaLon) * cos(toLat)
    let x = cos(fromLat) * sin(toLat) - sin(fromLat) * cos(toLat) * cos(deltaLon)
    return (degrees(atan2(y, x)) + 360).truncatingRemainder(dividingBy: 360)
}

func pointAt(_ centre: TrackPoint, _ metres: Double, _ bearingDegrees: Double) -> TrackPoint {
    let angular = metres / EARTH_RADIUS_M
    let lat = radians(centre.latitude)
    let lon = radians(centre.longitude)
    let bearing = radians(bearingDegrees)
    let pointLat = asin(sin(lat) * cos(angular) + cos(lat) * sin(angular) * cos(bearing))
    let pointLon = lon + atan2(sin(bearing) * sin(angular) * cos(lat), cos(angular) - sin(lat) * sin(pointLat))
    return TrackPoint(latitude: degrees(pointLat), longitude: degrees(pointLon))
}

func circlesAsPolygons(_ circles: [FenceCircle]) -> [FencePolygon] {
    circles.compactMap { circle in
        let ring = circleRing(circle.centre, circle.radiusMetres)
        return ring.count < 3 ? nil : FencePolygon(index: circle.index, inclusion: circle.inclusion, vertices: ring, kindText: circle.kindText)
    }
}
