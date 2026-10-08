import Foundation

private let EARTH_RADIUS_METRES = 6_371_000.0
private let MIN_SPAN_METRES = 1.0

struct Clearance: Equatable {
    var collides: Bool
    var metres: Double?
    var text: String
    var complete: Bool
}

struct ProfilePoint: Equatable {
    var distance: Double
    var terrain: Double?
    var planned: Double
    var collision: Bool = false
}

struct ProfileMarker: Equatable {
    var sequence: Int
    var distance: Double
    var label: String
    var endDistance: Double? = nil
    var lastSequence: Int? = nil
    var pattern: String = ""
}

struct TerrainProfile: Equatable {
    var points: [ProfilePoint]
    var clearance: Clearance? = nil
    var lowestText: String = ""
    var highestText: String = ""
    var distanceText: String = ""
    var bandText: String = ""
    var markers: [ProfileMarker] = []
    var band: (Double, Double)? = nil
    var heightHeader: String = ""
    var distanceTicks: [String] = []
    var heightTicks: [String] = []

    static func == (lhs: TerrainProfile, rhs: TerrainProfile) -> Bool {
        lhs.points == rhs.points && lhs.clearance == rhs.clearance && lhs.lowestText == rhs.lowestText
            && lhs.highestText == rhs.highestText && lhs.distanceText == rhs.distanceText && lhs.bandText == rhs.bandText
            && lhs.markers == rhs.markers && lhs.band?.0 == rhs.band?.0 && lhs.band?.1 == rhs.band?.1
            && lhs.heightHeader == rhs.heightHeader && lhs.distanceTicks == rhs.distanceTicks && lhs.heightTicks == rhs.heightTicks
    }

    var distance: Double { points.last?.distance ?? 0 }

    private var plannedPoints: [ProfilePoint] { points.filter { $0.planned.isFinite } }

    var lowest: Double {
        band?.0 ?? plannedPoints.map { min($0.terrain ?? $0.planned, $0.planned) }.min() ?? 0
    }

    var highest: Double {
        band?.1 ?? plannedPoints.map { max($0.terrain ?? $0.planned, $0.planned) }.max() ?? 0
    }

    var hasTerrain: Bool { points.filter { $0.terrain != nil }.count >= 2 }

    var terrainCoverage: Double {
        guard distance > 0 else { return 0 }
        return zip(points, points.dropFirst())
            .filter { from, to in from.terrain != nil && to.terrain != nil }
            .map { from, to in to.distance - from.distance }
            .reduce(0, +) / distance
    }

    var span: Double { max(highest - lowest, MIN_SPAN_METRES) }

    var flat: Bool { highest - lowest < MIN_SPAN_METRES }

    var drawable: Bool { points.count >= 2 }
}

func metresBetween(_ from: TrackPoint, _ to: TrackPoint) -> Double {
    let fromLat = from.latitude * .pi / 180
    let toLat = to.latitude * .pi / 180
    let deltaLat = toLat - fromLat
    let deltaLon = (to.longitude - from.longitude) * .pi / 180
    let a = pow(sin(deltaLat / 2), 2) + cos(fromLat) * cos(toLat) * pow(sin(deltaLon / 2), 2)
    return 2 * EARTH_RADIUS_METRES * asin(min(1, sqrt(a)))
}

let TERRAIN_VIEW = "view.terrainProfile"

func clearanceOf(_ view: JSON?) -> Clearance? {
    guard let view else { return nil }
    let metres = view["minClearanceMetres"].double(.nan)
    return Clearance(
        collides: view["hasCollision"].bool,
        metres: metres.isNaN ? nil : metres,
        text: view["clearanceText"].isNull ? "" : view["clearanceText"].string,
        complete: view["clearanceComplete"].bool
    )
}

func terrainWarning(_ clearance: Clearance?) -> String? {
    guard let clearance else { return nil }
    guard clearance.collides || (clearance.metres.map { $0 < 0 } ?? false) else { return nil }
    return clearance.text.isBlank ? "The route goes below the ground." : "The route goes \(clearance.text) below the ground."
}

func terrainProfile(_ view: JSON?) -> TerrainProfile {
    guard let view, let points = view["points"].arrayOrNil else { return TerrainProfile(points: []) }
    let low = view["minAltitudeMeters"].double(.nan)
    let high = view["maxAltitudeMeters"].double(.nan)
    return TerrainProfile(
        points: points.filter { $0.object != nil }.map { point in
            let terrain = point["terrainAltitude"].double(.nan)
            return ProfilePoint(
                distance: point["distance"].double(0),
                terrain: terrain.isNaN ? nil : terrain,
                planned: point["missionAltitude"].double(.nan),
                collision: point["collision"].bool
            )
        },
        clearance: clearanceOf(view),
        lowestText: view["lowestText"].string,
        highestText: view["highestText"].string,
        distanceText: view["distanceText"].string,
        bandText: view["bandText"].string,
        markers: (view["markers"].arrayOrNil ?? []).filter { $0.object != nil }.map { marker in
            let complex = marker["complex"]
            let isComplex = complex.object != nil
            return ProfileMarker(
                sequence: marker["sequence"].int(0),
                distance: marker["distance"].double(0),
                label: marker["label"].string,
                endDistance: isComplex ? complex["endDistance"].double(.nan) : nil,
                lastSequence: isComplex ? complex["lastSequence"].int(0) : nil,
                pattern: isComplex ? complex["pattern"].string : ""
            )
        },
        band: low.isFinite && high.isFinite ? (low, high) : nil,
        heightHeader: view["heightHeader"].string,
        distanceTicks: view["distanceTicks"].arrayOrNil?.map(\.string) ?? [],
        heightTicks: view["heightTicks"].arrayOrNil?.map(\.string) ?? []
    )
}

func collidingItems(_ view: JSON?, _ key: String = "collidingItems") -> Set<Int> {
    Set((view?[key].arrayOrNil ?? []).map { $0.int(-1) }.filter { $0 >= 0 })
}

func collisionLegs(_ view: JSON?) -> [(TrackPoint, TrackPoint)] {
    func spot(_ json: JSON) -> TrackPoint? {
        guard json.object != nil else { return nil }
        let latitude = json["latitude"].double(.nan)
        let longitude = json["longitude"].double(.nan)
        return isPlottable(latitude, longitude) ? TrackPoint(latitude: latitude, longitude: longitude) : nil
    }
    return (view?["collisionLegs"].arrayOrNil ?? []).compactMap { leg in
        spot(leg["from"]).flatMap { from in spot(leg["to"]).map { (from, $0) } }
    }
}
