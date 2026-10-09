import MapLibre
import UIKit

let ORBIT_VIEW = "view.orbit"
private let ORBIT_SOURCE = "aircast-orbit"
private let ORBIT_RING_SOURCE = "aircast-orbit-ring"
private let ORBIT_LAYER = "aircast-orbit-layer"
private let ORBIT_LABEL_LAYER = "aircast-orbit-label"
let ORBIT_RING_LAYER = "aircast-orbit-ring-layer"
private let ORBIT_COLOUR = "#FFFFFF"
private let ORBIT_ARROW_SOURCE = "aircast-orbit-arrows"
private let ORBIT_ARROW_LAYER = "aircast-orbit-arrow-layer"
private let ARROW_BEARING = "bearing"
private let ORBIT_HANDLE_SOURCE = "aircast-orbit-handles"
private let ORBIT_HANDLE_LAYER = "aircast-orbit-handle-layer"

struct OrbitCircle: Equatable {
    var centre: TrackPoint
    var radiusMetres: Double
    var clockwise: Bool = true
}

func orbitArrows(_ orbit: OrbitCircle?) -> [(TrackPoint, Double)] {
    guard let circle = orbit else { return [] }
    return [(0.0, 90.0), (180.0, 270.0)].map { around, travel in
        (pointAt(circle.centre, circle.radiusMetres, around), circle.clockwise ? travel : (travel + 180).truncatingRemainder(dividingBy: 360))
    }
}

func orbitCircle(_ view: JSON?) -> OrbitCircle? {
    guard let turning = view, turning["orbiting"].bool else { return nil }
    let centre = turning["centre"]
    let latitude = centre["latitude"].double(.nan)
    let longitude = centre["longitude"].double(.nan)
    let radius = turning["radiusMetres"].double(.nan)
    guard centre.object != nil, isPlottable(latitude, longitude), !radius.isNaN, radius > 0 else { return nil }
    return OrbitCircle(
        centre: TrackPoint(latitude: latitude, longitude: longitude),
        radiusMetres: radius,
        clockwise: turning["clockwise"].bool(true)
    )
}

func orbitRing(_ orbit: OrbitCircle?) -> [TrackPoint] {
    let ring = orbit.map { circleRing($0.centre, $0.radiusMetres) } ?? []
    return ring.first.map { ring + [$0] } ?? []
}

func orbitRadiusHandle(_ orbit: OrbitCircle?) -> TrackPoint? { orbit.map { pointAt($0.centre, $0.radiusMetres, 90) } }

func orbitHandles(_ preview: OrbitCircle?) -> [TrackPoint] { [preview?.centre, orbitRadiusHandle(preview)].compactMap { $0 } }

func draggedOrbitRadius(_ orbit: OrbitCircle, _ to: TrackPoint) -> Double {
    max(metresBetween(orbit.centre, to), MINIMUM_CIRCLE_RADIUS_METRES)
}

enum OrbitBridge {
    static func read() -> OrbitCircle? { orbitCircle(Qgc.get(ORBIT_VIEW)) }
}

func installOrbitLayer(_ style: MLNStyle) {
    guard style.source(withIdentifier: ORBIT_SOURCE) == nil else { return }
    let centre = geoJsonSource(ORBIT_SOURCE)
    let ring = geoJsonSource(ORBIT_RING_SOURCE)
    let arrows = geoJsonSource(ORBIT_ARROW_SOURCE)
    let handles = geoJsonSource(ORBIT_HANDLE_SOURCE)
    [centre, ring, arrows, handles].forEach(style.addSource)
    let line = MLNLineStyleLayer(identifier: ORBIT_RING_LAYER, source: ring)
    line.lineColor = styleConstant(mapColour(ORBIT_COLOUR))
    line.lineWidth = styleConstant(2)
    style.addLayer(line)
    let arrow = MLNSymbolStyleLayer(identifier: ORBIT_ARROW_LAYER, source: arrows)
    arrow.text = styleConstant("\u{25B2}")
    arrow.textFontNames = styleConstant(["Noto Sans Regular"])
    arrow.textFontSize = styleConstant(14)
    arrow.textColor = styleConstant(mapColour(ORBIT_COLOUR))
    arrow.textRotation = styleGet(ARROW_BEARING)
    arrow.textRotationAlignment = styleConstant("map")
    arrow.textAllowsOverlap = styleConstant(true)
    arrow.textIgnoresPlacement = styleConstant(true)
    style.addLayer(arrow)
    let dot = MLNCircleStyleLayer(identifier: ORBIT_LAYER, source: centre)
    dot.circleColor = styleConstant(mapColour(ORBIT_COLOUR))
    dot.circleRadius = styleConstant(6)
    style.addLayer(dot)
    let label = MLNSymbolStyleLayer(identifier: ORBIT_LABEL_LAYER, source: centre)
    label.text = styleConstant("Orbit")
    label.textFontSize = styleConstant(12)
    label.textColor = styleConstant(UIColor.white)
    label.textOffset = styleOffset(0, 1.4)
    label.textAnchor = styleConstant("top")
    label.textAllowsOverlap = styleConstant(true)
    label.textIgnoresPlacement = styleConstant(true)
    style.addLayer(label)
    let handle = MLNCircleStyleLayer(identifier: ORBIT_HANDLE_LAYER, source: handles)
    handle.circleColor = styleConstant(UIColor.white)
    handle.circleRadius = styleConstant(9)
    handle.circleStrokeColor = styleConstant(UIColor.black)
    handle.circleStrokeWidth = styleConstant(2)
    style.addLayer(handle)
}

func renderOrbit(_ style: MLNStyle, _ active: OrbitCircle?, _ gotoShown: Bool, preview: OrbitCircle? = nil) {
    let orbit = preview ?? active
    style.setGeoJson(ORBIT_HANDLE_SOURCE, featureCollection(orbitHandles(preview).map { pointFeature($0) }))
    style.setGeoJson(
        ORBIT_SOURCE,
        featureCollection([orbit].compactMap { $0 }.filter { _ in preview != nil || !gotoShown }.map { pointFeature($0.centre) })
    )
    style.setGeoJson(
        ORBIT_ARROW_SOURCE,
        featureCollection(orbitArrows(orbit).map { at, bearing in pointFeature(at, attributes: [ARROW_BEARING: bearing]) })
    )
    style.setGeoJson(ORBIT_RING_SOURCE, featureCollection([orbitRing(orbit)].filter { !$0.isEmpty }.map { lineFeature($0) }))
}
