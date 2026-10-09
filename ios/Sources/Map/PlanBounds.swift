import Foundation

struct PlanBounds: Equatable {
    var south: Double
    var west: Double
    var north: Double
    var east: Double

    var centre: TrackPoint {
        TrackPoint(latitude: (south + north) / 2, longitude: normaliseLongitude(west + longitudeSpan / 2))
    }

    var longitudeSpan: Double { east >= west ? east - west : east - west + 360 }

    var spanDegrees: Double { max(north - south, longitudeSpan) }
}

func normaliseLongitude(_ degrees: Double) -> Double {
    let shifted = (degrees + 180).truncatingRemainder(dividingBy: 360)
    let wrapped = (shifted < 0 ? shifted + 360 : shifted) - 180
    return wrapped == -180 ? 180 : wrapped
}

func longitudeArc(_ longitudes: [Double]) -> (Double, Double) {
    let sorted = longitudes.map(normaliseLongitude).sorted()
    let gaps = sorted.indices.map { index in
        ((index == sorted.count - 1 ? sorted[0] + 360 : sorted[index + 1]) - sorted[index], index)
    }
    guard let widest = gaps.max(by: { $0.0 < $1.0 }) else { return (0, 0) }
    return (sorted[(widest.1 + 1) % sorted.count], sorted[widest.1])
}

func planPoints(
    _ items: [MissionItem] = [],
    _ polygons: [FencePolygon] = [],
    _ circles: [FenceCircle] = [],
    _ rally: [RallyPoint] = [],
    _ surveys: [Survey] = []
) -> [TrackPoint] {
    items.map { TrackPoint(latitude: $0.latitude, longitude: $0.longitude) }
        + polygons.flatMap(\.vertices)
        + circlesAsPolygons(circles).flatMap(\.vertices)
        + rally.map { TrackPoint(latitude: $0.latitude, longitude: $0.longitude) }
        + surveys.flatMap { $0.area + $0.transects }
}

func fitPoints(_ plan: [TrackPoint], _ latitude: Double, _ longitude: Double) -> [TrackPoint] {
    guard plan.isEmpty else { return plan }
    return isPlottable(latitude, longitude) ? [TrackPoint(latitude: latitude, longitude: longitude)] : []
}

func planBounds(_ points: [TrackPoint]) -> PlanBounds? {
    let usable = points.filter { isPlottable($0.latitude, $0.longitude) }
    guard let south = usable.map(\.latitude).min(), let north = usable.map(\.latitude).max() else { return nil }
    let (west, east) = longitudeArc(usable.map(\.longitude))
    return PlanBounds(south: south, west: west, north: north, east: east)
}

func planIsDrawn(
    _ items: [MissionItem],
    _ surveys: [Survey],
    _ polygons: [FencePolygon],
    _ circles: [FenceCircle],
    _ rally: [RallyPoint]
) -> Bool {
    !items.isEmpty || !surveys.isEmpty || !polygons.isEmpty || !circles.isEmpty || !rally.isEmpty
}

func fitsPlanOnEntry(_ firstRead: Bool, _ planRead: Bool, _ planIsDrawn: Bool) -> Bool {
    firstRead && planRead && planIsDrawn
}

func centersOnVehicleAtEntry(_ alreadyCentred: Bool, _ vehicleKnown: Bool, _ fitsRequested: Int) -> Bool {
    !alreadyCentred && vehicleKnown && fitsRequested == 0
}

func stillFirstRead(_ firstRead: Bool, _ planRead: Bool) -> Bool { firstRead && !planRead }

func clearWindow(_ corners: [TrackPoint]) -> [TrackPoint] {
    let latitudes = corners.map(\.latitude)
    let longitudes = corners.map(\.longitude)
    let north = latitudes.max() ?? .nan
    let south = latitudes.min() ?? .nan
    let west = longitudes.min() ?? .nan
    let east = longitudes.max() ?? .nan
    return [
        TrackPoint(latitude: north, longitude: west),
        TrackPoint(latitude: north, longitude: east),
        TrackPoint(latitude: south, longitude: east),
        TrackPoint(latitude: south, longitude: west),
    ]
}
