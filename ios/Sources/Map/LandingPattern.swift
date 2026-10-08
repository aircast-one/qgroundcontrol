import Foundation

struct LandingPattern: Equatable {
    var index: Int
    var landing: TrackPoint?
    var slopeStart: TrackPoint?
    var finalApproach: TrackPoint?
    var loiterRadiusMetres: Double?
    var loiterClockwise: Bool
    var loiterRadiusText: String = ""
    var loiterToAltitude: Bool = false
    var heights: GlideSlopeHeights? = nil
    var collides: Bool = false
    var glideSlopeShown: Bool = true
}

private func place(_ view: JSON?, _ key: String) -> TrackPoint? {
    guard let at = view?[key], at.object != nil else { return nil }
    let latitude = at["latitude"].double(.nan)
    let longitude = at["longitude"].double(.nan)
    return latitude.isNaN || longitude.isNaN ? nil : TrackPoint(latitude: latitude, longitude: longitude)
}

struct GlideSlopeHeights: Equatable {
    var transition: String
    var midSlope: String
    var approach: String
}

struct LandingLabel: Equatable {
    var at: TrackPoint
    var text: String
}

func landingLabels(_ pattern: LandingPattern) -> [LandingLabel] {
    guard pattern.glideSlopeShown, let landing = pattern.landing, let slopeStart = pattern.slopeStart else { return [] }
    let bearing = azimuthBetween(landing, slopeStart)
    let side = bearing + (bearing > 180 ? -90 : 90)
    let transition = pointAt(landing, LANDING_LENGTH_M / 2, bearing)
    let top = (pattern.loiterToAltitude ? slopeStart : pattern.finalApproach) ?? slopeStart
    let mid = pointAt(transition, metresBetween(transition, top) / 2, bearing)
    let heights = pattern.heights
    return [
        LandingLabel(at: landing, text: "Landing Area"),
        LandingLabel(at: pointAt(landing, LANDING_LENGTH_M / 2 + 2, bearing), text: "Glide Slope"),
        heights.map { LandingLabel(at: pointAt(transition, LANDING_WIDTH_M, side), text: $0.transition) },
        heights.map { LandingLabel(at: pointAt(mid, LANDING_WIDTH_M / 2, side), text: $0.midSlope) },
        heights.map { LandingLabel(at: slopeStart, text: $0.approach) },
    ].compactMap { $0 }
}

func structureScanLabels(_ items: [MissionItem]) -> [LandingLabel] {
    items.filter { $0.kind == KIND_STRUCTURE }.flatMap { item in
        let entry = TrackPoint(latitude: item.latitude, longitude: item.longitude)
        return [LandingLabel(at: entry, text: "Entry"), LandingLabel(at: item.exit ?? entry, text: "Exit")]
    }
}

func landingLabelFeatures(_ patterns: [LandingPattern], _ selected: Int?, items: [MissionItem] = []) -> FeatureCollection {
    featureCollection(
        (patterns.filter { $0.index == selected }.flatMap(landingLabels) + structureScanLabels(items)).map { label in
            pointFeature(label.at, attributes: [LANDING_LABEL_TEXT: label.text])
        }
    )
}

func isLandingPattern(_ view: JSON?) -> Bool {
    guard let fields = view?.object else { return false }
    let kind = fields["kind"]
    return kind != .null && kind != .string("null") && fields["reason"].map(\.isNull) ?? true
}

func landingPattern(_ index: Int, _ view: JSON?) -> LandingPattern? {
    guard let view, isLandingPattern(view) else { return nil }
    let radius = view["loiterRadiusMetres"].double(.nan)
    let heights = view["heights"]
    let pattern = LandingPattern(
        index: index,
        landing: place(view, "landing"),
        slopeStart: place(view, "slopeStart"),
        finalApproach: place(view, "finalApproach"),
        loiterRadiusMetres: radius.isNaN ? nil : radius,
        loiterClockwise: view["loiterClockwise"].bool,
        loiterRadiusText: view["loiterRadiusText"].string,
        loiterToAltitude: view["loiterToAltitude"].bool,
        heights: heights.object == nil ? nil : GlideSlopeHeights(
            transition: heights["transition"].string,
            midSlope: heights["midSlope"].string,
            approach: heights["approach"].string
        ),
        glideSlopeShown: view["glideSlopeShown"].bool(true)
    )
    return pattern.landing != nil || pattern.slopeStart != nil || pattern.finalApproach != nil ? pattern : nil
}

func landingText(_ pattern: LandingPattern?) -> String? {
    guard let pattern, !pattern.loiterRadiusText.isBlank else { return nil }
    return "circles \(pattern.loiterRadiusText) \(pattern.loiterClockwise ? "clockwise" : "anticlockwise")"
}

func approachPath(_ pattern: LandingPattern) -> [TrackPoint] {
    [pattern.finalApproach, pattern.slopeStart, pattern.landing].compactMap { $0 }
}

func landingPathFeatures(_ patterns: [LandingPattern]) -> FeatureCollection {
    featureCollection(patterns.map(approachPath).filter { $0.count >= 2 }.map { lineFeature($0) })
}

private let LANDING_WIDTH_M = 15.0
private let LANDING_LENGTH_M = 100.0
let LANDING_AREA_KIND = "area"
let GLIDE_SLOPE_KIND = "slope"

private func landingCorners(_ landing: TrackPoint, _ bearing: Double) -> [TrackPoint] {
    let angle = atan((LANDING_WIDTH_M / 2) / (LANDING_LENGTH_M / 2)) * 180 / .pi
    let hypotenuse = (LANDING_WIDTH_M / 2) / sin(angle * .pi / 180)
    return [bearing - angle, bearing + angle, bearing + (180 - angle), bearing - (180 - angle)].map { pointAt(landing, hypotenuse, $0) }
}

func landingArea(_ pattern: LandingPattern) -> [TrackPoint]? {
    guard let landing = pattern.landing, let slopeStart = pattern.slopeStart else { return nil }
    return landingCorners(landing, azimuthBetween(landing, slopeStart))
}

func glideSlope(_ pattern: LandingPattern) -> [TrackPoint]? {
    guard let landing = pattern.landing, let slopeStart = pattern.slopeStart,
          let top = pattern.loiterToAltitude ? slopeStart : pattern.finalApproach else { return nil }
    return Array(landingCorners(landing, azimuthBetween(landing, slopeStart)).prefix(2)) + [top]
}

func landingAreaFeatures(_ patterns: [LandingPattern]) -> FeatureCollection {
    featureCollection(
        patterns.filter(\.glideSlopeShown).flatMap { pattern in
            [
                landingArea(pattern).map { (LANDING_AREA_KIND, $0, false) },
                glideSlope(pattern).map { (GLIDE_SLOPE_KIND, $0, pattern.collides) },
            ].compactMap { $0 }
        }.map { kind, ring, collides in
            polygonFeature(ring + [ring[0]], attributes: [LANDING_SHAPE_KIND: kind, TERRAIN_COLLISION: collides])
        }
    )
}

func loiterRings(_ patterns: [LandingPattern], _ items: [MissionItem]) -> [(TrackPoint, Double)] {
    loiterCircles(patterns, items).map { centre, radius, _ in (centre, radius) }
}

private func loiterCircles(_ patterns: [LandingPattern], _ items: [MissionItem]) -> [(TrackPoint, Double, Bool)] {
    patterns.filter(\.loiterToAltitude).compactMap { pattern in
        pattern.finalApproach.flatMap { centre in pattern.loiterRadiusMetres.map { (centre, $0, false) } }
    } + items.filter { $0.loiterRadius.isFinite && $0.loiterRadius != 0 }.map {
        (TrackPoint(latitude: $0.latitude, longitude: $0.longitude), abs($0.loiterRadius), $0.terrainCollision)
    }
}

func landingLoiterFeatures(_ patterns: [LandingPattern], items: [MissionItem] = []) -> FeatureCollection {
    featureCollection(
        loiterCircles(patterns, items).compactMap { centre, radius, collides in
            let ring = circleRing(centre, radius)
            return ring.count >= 3 ? lineFeature(ring + [ring[0]], attributes: [TERRAIN_COLLISION: collides]) : nil
        }
    )
}
