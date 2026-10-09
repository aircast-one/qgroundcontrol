import MapLibre
import UIKit

private let RADAR_SOURCE = "aircast-proximity-radar"
private let RADAR_LAYER = "aircast-proximity-radar-layer"
private let COLOUR_PROPERTY = "colour"
private let LIMIT_COLOUR = "#FFFFFF"
private let SECTOR_COLOUR = "#FF0000"
private let RADAR_OPACITY = 0.5
private let RADAR_LINE_WIDTH = 3.0
private let SECTOR_SWEEP_DEGREES = 45.0
private let ARC_STEPS = 9
private let LIMIT_SEGMENTS = 64

struct RadarReading: Equatable {
    var maxMeters: Double?
    var sectors: [(Double, Double)]

    static func == (lhs: RadarReading, rhs: RadarReading) -> Bool {
        lhs.maxMeters == rhs.maxMeters && lhs.sectors.elementsEqual(rhs.sectors) { sameDouble($0.0, $1.0) && sameDouble($0.1, $1.1) }
    }
}

func radarReading(_ view: JSON?) -> RadarReading? {
    guard let view, view["shown"].bool else { return nil }
    return RadarReading(
        maxMeters: view["maxMeters"].isNull ? nil : view["maxMeters"].double.flatMap { $0.isFinite ? $0 : nil },
        sectors: (view["sectors"].arrayOrNil ?? [])
            .filter { $0.object != nil && !$0["meters"].isNull }
            .map { ($0["bearing"].double(.nan), $0["meters"].double(.nan)) }
    )
}

func radarLines(_ centre: TrackPoint, _ heading: Double, _ reading: RadarReading) -> [(String, [TrackPoint])] {
    let facing = heading.isNaN ? 0 : heading
    let limit = reading.maxMeters.map { metres in
        let ring = circleRing(centre, metres, LIMIT_SEGMENTS)
        return ring + ring.prefix(1)
    }
    let arcs = reading.sectors.filter { _, metres in metres > 0 }.map { bearing, metres in
        let start = facing + bearing - SECTOR_SWEEP_DEGREES / 2
        return (SECTOR_COLOUR, (0...ARC_STEPS).map { step in pointAt(centre, metres, start + SECTOR_SWEEP_DEGREES * Double(step) / Double(ARC_STEPS)) })
    }
    return (limit.map { [(LIMIT_COLOUR, $0)] } ?? []) + arcs
}

func installProximityRadarLayer(_ style: MLNStyle) {
    guard style.source(withIdentifier: RADAR_SOURCE) == nil else { return }
    let source = geoJsonSource(RADAR_SOURCE)
    style.addSource(source)
    let layer = MLNLineStyleLayer(identifier: RADAR_LAYER, source: source)
    layer.lineColor = styleGet(COLOUR_PROPERTY)
    layer.lineWidth = styleConstant(RADAR_LINE_WIDTH)
    layer.lineOpacity = styleConstant(RADAR_OPACITY)
    style.addLayer(layer)
}

struct PlacedRadar: Equatable {
    var at: TrackPoint
    var heading: Double
    var reading: RadarReading

    static func == (lhs: PlacedRadar, rhs: PlacedRadar) -> Bool {
        lhs.at == rhs.at && sameDouble(lhs.heading, rhs.heading) && lhs.reading == rhs.reading
    }
}

func placedRadars(_ fleet: [VehicleChoice], _ active: TrackPoint, _ activeHeading: Double) -> [PlacedRadar] {
    fleet.compactMap { vehicle in
        vehicle.radar.map { reading in
            vehicle.active
                ? PlacedRadar(at: active, heading: activeHeading, reading: reading)
                : PlacedRadar(at: TrackPoint(latitude: vehicle.latitude, longitude: vehicle.longitude), heading: vehicle.heading, reading: reading)
        }
    }.filter { isPlottable($0.at.latitude, $0.at.longitude) }
}

func renderProximityRadars(_ style: MLNStyle, _ radars: [PlacedRadar]) {
    let lines = radars.flatMap { radarLines($0.at, $0.heading, $0.reading) }
    style.setGeoJson(RADAR_SOURCE, featureCollection(lines.map { colour, points in lineFeature(points, attributes: [COLOUR_PROPERTY: colour]) }))
}
