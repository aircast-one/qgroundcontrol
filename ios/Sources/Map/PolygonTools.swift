import Foundation
import MapLibre

private let DEFAULT_SHAPE_FRACTION = 0.75
private let DEFAULT_SHAPE_MAX_METRES = 3000.0
private let DEFAULT_CIRCLE_SEGMENTS = 16
private let METRES_PER_DEGREE = 111_320.0
private let TRACE_SOURCE = "aircast-polygon-trace"
private let TRACE_LINE_LAYER = "aircast-polygon-trace-line"
private let TRACE_DOT_LAYER = "aircast-polygon-trace-dots"
private let TRACE_COLOUR = "#FFFFFF"
private let TRACE_LINE_WIDTH = 2.0
private let TRACE_DOT_RADIUS = 4.0
private let STAGED_SHAPE_NAME = "shape"
private let UNREADABLE_FILE = "That file could not be read."

private func offset(_ centre: TrackPoint, _ northMetres: Double, _ eastMetres: Double) -> TrackPoint {
    TrackPoint(
        latitude: centre.latitude + northMetres / METRES_PER_DEGREE,
        longitude: centre.longitude + eastMetres / (METRES_PER_DEGREE * cos(centre.latitude * .pi / 180))
    )
}

private func halfExtents(_ view: [TrackPoint]) -> (Double, Double)? {
    guard view.count == 4 else { return nil }
    func half(_ metres: Double) -> Double { min(metres * DEFAULT_SHAPE_FRACTION, DEFAULT_SHAPE_MAX_METRES) / 2 }
    return (half(metresBetween(view[0], view[1])), half(metresBetween(view[0], view[3])))
}

private func viewCentre(_ view: [TrackPoint]) -> TrackPoint {
    TrackPoint(
        latitude: view.map(\.latitude).reduce(0, +) / Double(view.count),
        longitude: view.map(\.longitude).reduce(0, +) / Double(view.count)
    )
}

func defaultRectangle(_ view: [TrackPoint]) -> [TrackPoint] {
    guard let (halfWidth, halfHeight) = halfExtents(view) else { return [] }
    let centre = viewCentre(view)
    return [(halfHeight, -halfWidth), (halfHeight, halfWidth), (-halfHeight, halfWidth), (-halfHeight, -halfWidth)]
        .map { offset(centre, $0.0, $0.1) }
}

func defaultCircle(_ view: [TrackPoint]) -> [TrackPoint] {
    guard let (halfWidth, halfHeight) = halfExtents(view) else { return [] }
    return circleRing(viewCentre(view), min(halfWidth, halfHeight), DEFAULT_CIRCLE_SEGMENTS)
}

struct ShapeTarget: Equatable, Hashable {
    var path: String
    var line: Bool

    var minimum: Int { line ? 2 : 3 }
    var fileShape: String { line ? "polyline" : "polygon" }
    var missing: String { line ? "No polylines found in file" : "No polygons found in file" }
}

func shapeTarget(_ fence: Int?, _ survey: Survey?) -> ShapeTarget? {
    if let fence { return ShapeTarget(path: "\(FENCE_POLYGONS).\(fence)", line: false) }
    if let survey { return ShapeTarget(path: "\(PLAN_ITEMS).\(survey.index).\(survey.property)", line: survey.property == CORRIDOR_PROPERTY) }
    return nil
}

func defaultLine(_ view: [TrackPoint]) -> [TrackPoint] {
    guard view.count == 4 else { return [] }
    let (topLeft, topRight, bottomRight, bottomLeft) = (view[0], view[1], view[2], view[3])
    let top = TrackPoint(latitude: (topLeft.latitude + topRight.latitude) / 2, longitude: (topLeft.longitude + topRight.longitude) / 2)
    let bottom = TrackPoint(latitude: (bottomLeft.latitude + bottomRight.latitude) / 2, longitude: (bottomLeft.longitude + bottomRight.longitude) / 2)
    return [0.25, 0.75].map { fraction in
        TrackPoint(
            latitude: top.latitude + (bottom.latitude - top.latitude) * fraction,
            longitude: top.longitude + (bottom.longitude - top.longitude) * fraction
        )
    }
}

func polygonCentre(_ vertices: [TrackPoint]) -> TrackPoint? {
    guard vertices.count >= 3, let origin = vertices.first else { return nil }
    let local = vertices.map { ($0.longitude - origin.longitude, $0.latitude - origin.latitude) }
    let edges = local.indices.map { (local[$0], local[($0 + 1) % local.count]) }
    let cross = edges.map { a, b in a.0 * b.1 - b.0 * a.1 }
    let area = cross.reduce(0, +) / 2
    guard area != 0 else { return nil }
    let x = zip(edges, cross).map { edge, c in (edge.0.0 + edge.1.0) * c }.reduce(0, +) / (6 * area)
    let y = zip(edges, cross).map { edge, c in (edge.0.1 + edge.1.1) * c }.reduce(0, +) / (6 * area)
    return TrackPoint(latitude: origin.latitude + y, longitude: origin.longitude + x)
}

func shapeMovedTo(_ vertices: [TrackPoint], _ centre: TrackPoint) -> [TrackPoint]? {
    guard let from = polygonCentre(vertices) else { return nil }
    let distance = metresBetween(from, centre)
    let azimuth = azimuthBetween(from, centre)
    return vertices.map { pointAt($0, distance, azimuth) }
}

func fencePath(_ index: Int) -> String { "\(FENCE_POLYGONS).\(index)" }

func surveyPath(_ survey: Survey) -> String { "\(PLAN_ITEMS).\(survey.index).\(survey.property)" }

func circleRadius(_ vertices: [TrackPoint]) -> Double? {
    polygonCentre(vertices).flatMap { centre in vertices.first.map { metresBetween(centre, $0) } }
}

func circleAround(_ vertices: [TrackPoint], _ radius: Double) -> [TrackPoint]? {
    guard radius > 0, let centre = polygonCentre(vertices) else { return nil }
    return circleRing(centre, radius, DEFAULT_CIRCLE_SEGMENTS)
}

func shapeVertices(_ target: ShapeTarget, _ fences: [FencePolygon], _ surveys: [Survey]) -> [TrackPoint] {
    fences.first { fencePath($0.index) == target.path }?.vertices
        ?? surveys.first { surveyPath($0) == target.path }?.area
        ?? []
}

func shapeCentreHit(_ target: ShapeTarget, _ fences: [FencePolygon], _ surveys: [Survey]) -> (MapHit, TrackPoint)? {
    guard let centre = polygonCentre(shapeVertices(target, fences, surveys)) else { return nil }
    let hit: MapHit? = fences.first { fencePath($0.index) == target.path }.map { .ShapeCentre(fence: true, owner: $0.index) }
        ?? surveys.first { surveyPath($0) == target.path }.map { .ShapeCentre(fence: false, owner: $0.index) }
    return hit.map { ($0, centre) }
}

func positionTitle(_ hit: MapHit) -> String {
    if case .ShapeCentre = hit { return "Edit center position" }
    return "Edit vertex position"
}

func shapeEditable(_ target: ShapeTarget, _ fences: [FencePolygon], _ surveys: [Survey]) -> EditableShape? {
    fences.first { fencePath($0.index) == target.path }?.editable
        ?? surveys.first { surveyPath($0) == target.path }?.editable
}

func shapeCaption(_ shape: EditableShape, _ circled: Bool) -> String {
    circled && !shape.circleCaption.isBlank ? shape.circleCaption : shape.caption
}

func traceCaption(_ count: Int, _ minimum: Int) -> String {
    count >= minimum ? "\(count) points" : "Click the map to add points \u{00B7} \(count) of \(minimum)"
}

func circleRadiusMetres(_ typed: String, _ metresPerUnit: Double) -> Double? {
    typedNumber(typed).flatMap { $0 > 0 ? $0 * metresPerUnit : nil }
}

private let CIRCLE_TOLERANCE = 0.01

func isCircleShape(_ vertices: [TrackPoint]) -> Bool {
    guard let centre = polygonCentre(vertices), let radius = circleRadius(vertices) else { return false }
    return vertices.count == DEFAULT_CIRCLE_SEGMENTS && radius > 0
        && vertices.allSatisfy { abs(metresBetween(centre, $0) - radius) <= radius * CIRCLE_TOLERANCE }
}

func liveCircles(_ circled: Set<String>, _ fences: [FencePolygon], _ surveys: [Survey]) -> Set<String> {
    circled.filter { isCircleShape(shapeVertices(ShapeTarget(path: $0, line: false), fences, surveys)) }
}

func replaceShape(_ target: ShapeTarget, _ vertices: [TrackPoint]) -> Bool {
    vertices.count >= target.minimum
        && invokeOk("\(target.path).clear")
        && invokeOk("\(target.path).appendVertices", vertices.map { coordinateJson($0.latitude, $0.longitude) })
}

let CORRIDOR_PROPERTY = "corridorPolyline"

func fileShape(_ view: JSON?, _ target: ShapeTarget) -> ([TrackPoint], String) {
    guard let view else { return ([], target.missing) }
    if !view["error"].string.isBlank { return ([], view["error"].string) }
    if view["shape"].string != target.fileShape { return ([], target.missing) }
    let points = view["points"].objects.map {
        TrackPoint(latitude: $0["latitude"].double(.nan), longitude: $0["longitude"].double(.nan))
    }
    return (points, "")
}

func extensionOf(_ name: String) -> String {
    name.contains(".") ? String(name.split(separator: ".", omittingEmptySubsequences: false).last ?? "").lowercased() : ""
}

func mainShapeExtension(_ names: [String]) -> String? {
    let extensions = names.map(extensionOf)
    return ["shp", "kml"].first { extensions.contains($0) } ?? extensions.first
}

private func stagedShape(_ folder: URL, _ ext: String) -> URL {
    folder.appendingPathComponent("\(STAGED_SHAPE_NAME).\(ext)")
}

func stageShapeFiles(_ urls: [URL], _ folder: URL) -> URL? {
    let files = FileManager.default
    let named = urls.map { ($0, extensionOf($0.lastPathComponent)) }
    guard let main = mainShapeExtension(named.map { "x.\($0.1)" }) else { return nil }
    let stale = (try? files.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil)) ?? []
    stale.filter { $0.lastPathComponent.hasPrefix("\(STAGED_SHAPE_NAME).") }.forEach { try? files.removeItem(at: $0) }
    let copied = named.allSatisfy { url, ext in
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        let staged = stagedShape(folder, ext)
        try? files.removeItem(at: staged)
        return (try? files.copyItem(at: url, to: staged)) != nil
    }
    return copied ? stagedShape(folder, main) : nil
}

func importShapeFiles(_ urls: [URL], _ target: ShapeTarget) -> String? {
    guard let staged = stageShapeFiles(urls, FileManager.default.temporaryDirectory) else { return UNREADABLE_FILE }
    let view = "view.\(target.line ? "lineFile" : "areaFile")(\(staged.path))"
    let (vertices, error) = fileShape(MapBridge.read(view), target)
    if !error.isBlank { return error }
    return replaceShape(target, vertices) ? nil : target.missing
}

func traceOutline(_ points: [TrackPoint], _ line: Bool = false) -> [TrackPoint] {
    !line && points.count >= 3 ? points + [points[0]] : points
}

func installTraceLayer(_ style: MLNStyle) {
    guard style.source(withIdentifier: TRACE_SOURCE) == nil else { return }
    let source = geoJsonSource(TRACE_SOURCE)
    style.addSource(source)
    let line = MLNLineStyleLayer(identifier: TRACE_LINE_LAYER, source: source)
    line.lineColor = styleConstant(mapColour(TRACE_COLOUR))
    line.lineWidth = styleConstant(TRACE_LINE_WIDTH)
    style.addLayer(line)
    let dots = MLNCircleStyleLayer(identifier: TRACE_DOT_LAYER, source: source)
    dots.circleColor = styleConstant(mapColour(TRACE_COLOUR))
    dots.circleRadius = styleConstant(TRACE_DOT_RADIUS)
    style.addLayer(dots)
}

func traceFeatures(_ points: [TrackPoint], _ line: Bool) -> [Feature] {
    let outline = traceOutline(points, line)
    return (outline.count >= 2 ? [lineFeature(outline)] : []) + outline.map { pointFeature($0) }
}

func renderTrace(_ style: MLNStyle, _ points: [TrackPoint], _ line: Bool) {
    style.setGeoJson(TRACE_SOURCE, featureCollection(traceFeatures(points, line)))
}
