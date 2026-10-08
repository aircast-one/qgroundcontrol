import MapLibre
import UIKit

private let TRANSECT_ARROW_SOURCE = "aircast-transect-arrows"
private let TRANSECT_ARROW_LAYER = "aircast-transect-arrow-layer"
private let TRANSECT_STUB_SOURCE = "aircast-transect-stubs"
private let TRANSECT_STUB_LAYER = "aircast-transect-stub-layer"
private let TRANSECT_ARROW_IMAGE = "aircast-transect-arrow"
private let ARROW_BEARING = "bearing"
private let ARROW_SIZE_PX: CGFloat = 30
private let ENTRY_QUARTER = 1
private let EXIT_QUARTER = 3

struct TransectArrow: Equatable {
    var at: TrackPoint
    var bearing: Double
}

struct TransectMarks: Equatable {
    var arrows: [TransectArrow]
    var stubs: [[TrackPoint]]
}

func arrowOn(_ from: TrackPoint, _ to: TrackPoint, _ quarter: Int) -> TransectArrow {
    let at = pointAt(from, metresBetween(from, to) / 4 * Double(quarter), azimuthBetween(from, to))
    return TransectArrow(at: at, bearing: azimuthBetween(at, to))
}

func transectMarks(_ points: [TrackPoint], _ turnaround: Bool, _ current: Bool) -> TransectMarks {
    let step = turnaround ? 4 : 2
    let first = turnaround ? 1 : 0
    let last = points.count - (turnaround ? 2 : 1)
    let count = points.count / step
    let point = { (index: Int) in points.indices.contains(index) ? points[index] : nil }
    let segment = { (from: Int, quarter: Int) in point(from).flatMap { a in point(from + 1).map { b in arrowOn(a, b, quarter) } } }
    let arrows = !current || points.count < 2 ? [] : [
        segment(first, ENTRY_QUARTER),
        count > 3 ? segment(first + step, ENTRY_QUARTER) : nil,
        segment(last - 1, EXIT_QUARTER),
        count > 3 ? segment(last - step - 1, EXIT_QUARTER) : nil,
    ].compactMap { $0 }
    let stubs = current || !turnaround || points.count < 2 ? [] : [Array(points.prefix(2)), Array(points.suffix(2))]
    return TransectMarks(arrows: arrows, stubs: stubs)
}

private func chevron() -> UIImage {
    mapIcon(ARROW_SIZE_PX) { context in
        let half = ARROW_SIZE_PX / 2
        context.move(to: CGPoint(x: 0, y: half))
        context.addLine(to: CGPoint(x: half, y: 0))
        context.addLine(to: CGPoint(x: ARROW_SIZE_PX, y: half))
        context.setStrokeColor(UIColor.white.cgColor)
        context.setLineWidth(3)
        context.strokePath()
    }
}

private let GIMBAL_WEDGE_SOURCE = "aircast-gimbal-wedges"
private let GIMBAL_WEDGE_LAYER = "aircast-gimbal-wedge-layer"
private let GIMBAL_WEDGE_IMAGE = "aircast-gimbal-wedge"
private let GIMBAL_WEDGE_PX: CGFloat = 72
private let GIMBAL_WEDGE_SWEEP: CGFloat = 90

func gimbalWedges(_ items: [MissionItem]) -> [TransectArrow] {
    items.filter { $0.heading.isFinite && $0.gimbalYaw.isFinite }
        .map { TransectArrow(at: TrackPoint(latitude: $0.latitude, longitude: $0.longitude), bearing: $0.heading + $0.gimbalYaw) }
}

private func wedge() -> UIImage {
    mapIcon(GIMBAL_WEDGE_PX) { context in
        let centre = CGPoint(x: GIMBAL_WEDGE_PX / 2, y: GIMBAL_WEDGE_PX / 2)
        let start = (-90 - GIMBAL_WEDGE_SWEEP / 2) * .pi / 180
        context.move(to: centre)
        context.addArc(center: centre, radius: GIMBAL_WEDGE_PX / 2, startAngle: start, endAngle: start + GIMBAL_WEDGE_SWEEP * .pi / 180, clockwise: false)
        context.closePath()
        context.setFillColor(UIColor(white: 1, alpha: 140.0 / 255).cgColor)
        context.fillPath()
    }
}

func renderGimbalWedges(_ style: MLNStyle, _ items: [MissionItem]) {
    style.setGeoJson(
        GIMBAL_WEDGE_SOURCE,
        featureCollection(gimbalWedges(items).map { pointFeature($0.at, attributes: [ARROW_BEARING: $0.bearing]) })
    )
}

func installTransectMarks(_ style: MLNStyle) {
    if style.source(withIdentifier: GIMBAL_WEDGE_SOURCE) == nil {
        let source = geoJsonSource(GIMBAL_WEDGE_SOURCE)
        style.addSource(source)
        style.setImage(wedge(), forName: GIMBAL_WEDGE_IMAGE)
        let wedges = MLNSymbolStyleLayer(identifier: GIMBAL_WEDGE_LAYER, source: source)
        wedges.iconImageName = styleConstant(GIMBAL_WEDGE_IMAGE)
        wedges.iconRotation = styleGet(ARROW_BEARING)
        wedges.iconRotationAlignment = styleConstant("map")
        wedges.iconAllowsOverlap = styleConstant(true)
        wedges.iconIgnoresPlacement = styleConstant(true)
        style.addLayer(wedges)
    }
    guard style.source(withIdentifier: TRANSECT_ARROW_SOURCE) == nil else { return }
    let stubs = geoJsonSource(TRANSECT_STUB_SOURCE)
    style.addSource(stubs)
    let stubLine = MLNLineStyleLayer(identifier: TRANSECT_STUB_LAYER, source: stubs)
    stubLine.lineColor = styleConstant(UIColor.white)
    stubLine.lineWidth = styleConstant(2)
    style.addLayer(stubLine)
    let arrows = geoJsonSource(TRANSECT_ARROW_SOURCE)
    style.addSource(arrows)
    style.setImage(chevron(), forName: TRANSECT_ARROW_IMAGE)
    let arrowLayer = MLNSymbolStyleLayer(identifier: TRANSECT_ARROW_LAYER, source: arrows)
    arrowLayer.iconImageName = styleConstant(TRANSECT_ARROW_IMAGE)
    arrowLayer.iconRotation = styleGet(ARROW_BEARING)
    arrowLayer.iconRotationAlignment = styleConstant("map")
    arrowLayer.iconAllowsOverlap = styleConstant(true)
    arrowLayer.iconIgnoresPlacement = styleConstant(true)
    style.addLayer(arrowLayer)
}

func renderTransectMarks(_ style: MLNStyle, _ surveys: [Survey], _ selected: Int?, legArrows: [TransectArrow] = []) {
    let marks = surveys.map { transectMarks($0.transects, $0.turnaround, $0.index == selected) }
    style.setGeoJson(
        TRANSECT_ARROW_SOURCE,
        featureCollection((marks.flatMap(\.arrows) + legArrows).map { pointFeature($0.at, attributes: [ARROW_BEARING: $0.bearing]) })
    )
    style.setGeoJson(TRANSECT_STUB_SOURCE, featureCollection(marks.flatMap(\.stubs).map { lineFeature($0) }))
}
