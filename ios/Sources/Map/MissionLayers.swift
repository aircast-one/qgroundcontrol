import MapLibre
import UIKit

let CROWDED_ITEMS = 40
let CROWDED_RADIUS = 4.0
let MARKER_RADIUS = 13.0
private let CROWDED_STROKE = 0.5
private let MARKER_STROKE = 2.0
private let SELECTED_STROKE = 5.0
let WAYPOINT_RADIUS_PROPERTY = "waypointRadius"
let WAYPOINT_STROKE_PROPERTY = "waypointStroke"

let MISSION_SOURCE = "aircast-mission"
let MISSION_LAYER = "aircast-mission-layer"
let MISSION_DOT_LAYER = "aircast-mission-dot-layer"
let MISSION_PATH_SOURCE = "aircast-mission-path"
let MISSION_PATH_LAYER = "aircast-mission-path-layer"
private let COLLISION_LEG_SOURCE = "aircast-collision-legs"
private let COLLISION_LEG_LAYER = "aircast-collision-leg-layer"
private let COLLISION_COLOUR = "#FF0000"

func collisionLegFeatures(_ legs: [(TrackPoint, TrackPoint)]) -> FeatureCollection {
    featureCollection(legs.map { from, to in lineFeature([from, to]) })
}

func renderCollisionLegs(_ style: MLNStyle, _ legs: [(TrackPoint, TrackPoint)]) {
    style.setGeoJson(COLLISION_LEG_SOURCE, collisionLegFeatures(legs))
}

let WAYPOINT_ID_PROPERTY = "waypointId"
let WAYPOINT_LABEL_PROPERTY = "label"
let WAYPOINT_SIDE_LABEL_PROPERTY = "sideLabel"
private let MISSION_SIDE_LABEL_LAYER = "aircast-mission-side-label-layer"
let WAYPOINT_COLOUR_PROPERTY = "waypointColour"
let WAYPOINT_SELECTED_PROPERTY = "waypointSelected"

let TAKEOFF_COLOUR = "#43A047"
let LAND_COLOUR = "#E53935"
let RETURN_COLOUR = "#5C6BC0"
let LOITER_COLOUR = "#26A69A"
let START_COLOUR = "#7E57C2"
let WAYPOINT_COLOUR = "#FFB300"

let MAV_CMD_NAV_LOITER_UNLIM = 17
let MAV_CMD_NAV_LOITER_TURNS = 18
let MAV_CMD_NAV_LOITER_TIME = 19
let MAV_CMD_DO_ORBIT = 34

private let LOITER_COMMANDS: Set<Int> = [MAV_CMD_NAV_LOITER_UNLIM, MAV_CMD_NAV_LOITER_TURNS, MAV_CMD_NAV_LOITER_TIME, MAV_CMD_DO_ORBIT]

func waypointColour(_ kind: String, _ commandId: Int) -> String {
    switch true {
    case kind == "settings": START_COLOUR
    case kind == "takeoff": TAKEOFF_COLOUR
    case kind == "land": LAND_COLOUR
    case commandId == MAV_CMD_NAV_RETURN_TO_LAUNCH: RETURN_COLOUR
    case LOITER_COMMANDS.contains(commandId): LOITER_COLOUR
    default: WAYPOINT_COLOUR
    }
}

private let NOTO_SANS: [String] = ["Noto Sans Regular"]

func installMissionLayers(_ style: MLNStyle) {
    if style.source(withIdentifier: MISSION_PATH_SOURCE) == nil {
        let source = geoJsonSource(MISSION_PATH_SOURCE)
        style.addSource(source)
        let path = MLNLineStyleLayer(identifier: MISSION_PATH_LAYER, source: source)
        path.lineColor = styleConstant(mapColour("#FFB300"))
        path.lineWidth = styleConstant(3)
        path.lineDashPattern = styleConstant([2, 1.5])
        style.addLayer(path)
    }
    if style.source(withIdentifier: COLLISION_LEG_SOURCE) == nil {
        let source = geoJsonSource(COLLISION_LEG_SOURCE)
        style.addSource(source)
        let legs = MLNLineStyleLayer(identifier: COLLISION_LEG_LAYER, source: source)
        legs.lineColor = styleConstant(mapColour(COLLISION_COLOUR))
        legs.lineWidth = styleConstant(3)
        style.addLayer(legs)
    }

    guard style.source(withIdentifier: MISSION_SOURCE) == nil else { return }
    let source = geoJsonSource(MISSION_SOURCE)
    style.addSource(source)
    let dots = MLNCircleStyleLayer(identifier: MISSION_DOT_LAYER, source: source)
    dots.circleColor = styleGet(WAYPOINT_COLOUR_PROPERTY)
    dots.circleRadius = styleGet(WAYPOINT_RADIUS_PROPERTY)
    dots.circleStrokeColor = styleExpression(["case", ["get", WAYPOINT_SELECTED_PROPERTY], "#FFFFFF", "#37474F"])
    dots.circleStrokeWidth = styleGet(WAYPOINT_STROKE_PROPERTY)
    style.addLayer(dots)
    let labels = MLNSymbolStyleLayer(identifier: MISSION_LAYER, source: source)
    labels.text = styleGet(WAYPOINT_LABEL_PROPERTY)
    labels.textFontNames = styleConstant(NOTO_SANS)
    labels.textFontSize = styleConstant(15)
    labels.textColor = styleConstant(UIColor.white)
    labels.textHaloColor = styleConstant(mapColour("#37474F"))
    labels.textHaloWidth = styleConstant(2.5)
    labels.textAllowsOverlap = styleConstant(false)
    labels.textIgnoresPlacement = styleConstant(false)
    style.addLayer(labels)
    let sides = MLNSymbolStyleLayer(identifier: MISSION_SIDE_LABEL_LAYER, source: source)
    sides.text = styleGet(WAYPOINT_SIDE_LABEL_PROPERTY)
    sides.textFontNames = styleConstant(NOTO_SANS)
    sides.textFontSize = styleConstant(13)
    sides.textColor = styleConstant(UIColor.white)
    sides.textHaloColor = styleConstant(mapColour("#37474F"))
    sides.textHaloWidth = styleConstant(2)
    sides.textAnchor = styleConstant("left")
    sides.textOffset = styleOffset(1.2, 0)
    sides.textAllowsOverlap = styleConstant(true)
    style.addLayer(sides)
}

func crowded(_ itemCount: Int) -> Bool { itemCount > CROWDED_ITEMS }

func waypointLabel(_ sequence: Int, _ crowded: Bool, abbreviation: String = "") -> String {
    crowded ? "" : lettered(abbreviation) ? String(abbreviation.prefix(1)) : String(sequence)
}

func sideLabel(_ crowded: Bool, _ abbreviation: String) -> String {
    !crowded && lettered(abbreviation) && abbreviation.count > 1 ? abbreviation : ""
}

private func lettered(_ abbreviation: String) -> Bool {
    abbreviation.utf16.first.map { $0 > 65 && $0 < 122 } == true
}

func markerRadius(_ crowded: Bool, _ selected: Bool) -> Double { crowded && !selected ? CROWDED_RADIUS : MARKER_RADIUS }

func markerStroke(_ crowded: Bool, _ selected: Bool) -> Double {
    selected ? SELECTED_STROKE : crowded ? CROWDED_STROKE : MARKER_STROKE
}

private struct Marker {
    let item: MissionItem
    let at: TrackPoint
    let sequence: Int
    let exit: Bool
    var side: String? = nil
}

func exitMarkers(_ items: [MissionItem]) -> [(MissionItem, TrackPoint)] {
    items.filter(\.complexPattern).compactMap { item in item.exit.map { (item, $0) } }
}

private func landingMarkers(_ item: MissionItem, _ pattern: LandingPattern) -> [Marker] {
    [
        pattern.finalApproach.map { Marker(item: item, at: $0, sequence: item.sequence, exit: false, side: pattern.loiterToAltitude ? "Loiter" : "Approach") },
        pattern.landing.map { Marker(item: item, at: $0, sequence: item.sequence + item.foldedCommands, exit: true, side: "Land") },
    ].compactMap { $0 }
}

func missionFeatures(_ items: [MissionItem], selectedIndex: Int? = nil, landings: [LandingPattern] = []) -> FeatureCollection {
    let isCrowded = crowded(items.count)
    let patterns = Dictionary(landings.map { ($0.index, $0) }, uniquingKeysWith: { _, last in last })
    let plain = items.filter { patterns[$0.index] == nil }
    let markers = plain.map { Marker(item: $0, at: TrackPoint(latitude: $0.latitude, longitude: $0.longitude), sequence: $0.sequence, exit: false) }
        + exitMarkers(plain).map { item, exit in Marker(item: item, at: exit, sequence: item.sequence + item.foldedCommands, exit: true) }
        + items.flatMap { item in patterns[item.index].map { landingMarkers(item, $0) } ?? [] }
    return featureCollection(markers.map { marker in
        let lettered = !marker.exit && !marker.item.complexPattern ? marker.item.abbreviation : ""
        let selected = marker.item.index == selectedIndex
        return pointFeature(marker.at, attributes: [
            WAYPOINT_ID_PROPERTY: marker.item.index,
            WAYPOINT_LABEL_PROPERTY: waypointLabel(marker.sequence, isCrowded, abbreviation: lettered),
            WAYPOINT_SIDE_LABEL_PROPERTY: (isCrowded ? nil : marker.side) ?? sideLabel(isCrowded, lettered),
            WAYPOINT_RADIUS_PROPERTY: markerRadius(isCrowded, selected),
            WAYPOINT_STROKE_PROPERTY: markerStroke(isCrowded, selected),
            WAYPOINT_COLOUR_PROPERTY: waypointColour(marker.item.kind, marker.item.commandId),
            WAYPOINT_SELECTED_PROPERTY: selected,
        ])
    })
}

private let LEG_ARROW_QUARTER = 3
private let LEG_ARROW_SPACING = 5

private struct ArrowWalk {
    let arrows: [Bool]
    let count: Int
}

func flownRoute(_ items: [MissionItem], _ linkStartToHome: Bool) -> [MissionItem] {
    let flown = (linkStartToHome ? items : items.filter { $0.index != 0 }).filter { $0.routed && ($0.index != 0 || $0.placed) }
    guard let home = items.first(where: { $0.index == 0 && $0.closesRoute && $0.placed }), flown.contains(where: { $0.index != 0 }) else {
        return flown
    }
    return flown + [withChanges(home) { $0.exit = nil }]
}

func legArrows(_ items: [MissionItem], _ linkStartToHome: Bool) -> [TransectArrow] {
    let flown = flownRoute(items, linkStartToHome)
    let legs = Array(zip(flown, flown.dropFirst()))
    let walk = legs.enumerated().reduce(ArrowWalk(arrows: [], count: 0)) { walk, entry in
        let (leg, (from, to)) = entry
        guard to.index != 1 else { return ArrowWalk(arrows: walk.arrows + [false], count: walk.count) }
        let boundary = (leg == 0 && from.index == 0) || from.complexPattern || to.complexPattern
        let spaced = !boundary && walk.count > LEG_ARROW_SPACING
        return ArrowWalk(arrows: walk.arrows + [boundary || spaced], count: spaced ? 1 : walk.count + 1)
    }
    let marked = walk.arrows.enumerated().map { leg, arrow in arrow || leg == legs.count - 1 }
    return zip(legs, marked).filter { leg, arrow in arrow && !leg.1.legBroken }.map { leg, _ in
        let (from, to) = leg
        return arrowOn(from.exit ?? TrackPoint(latitude: from.latitude, longitude: from.longitude), TrackPoint(latitude: to.latitude, longitude: to.longitude), LEG_ARROW_QUARTER)
    }
}

func missionPath(_ items: [MissionItem], _ linkStartToHome: Bool) -> Feature? {
    let runs = flownRoute(items, linkStartToHome).reduce([[MissionItem]]()) { runs, item in
        item.legBroken || runs.isEmpty ? runs + [[item]] : Array(runs.dropLast()) + [(runs.last ?? []) + [item]]
    }
    let lines = runs.map { run in
        run.flatMap { item in [TrackPoint(latitude: item.latitude, longitude: item.longitude)] + [item.exit].compactMap { $0 } }
    }.filter { $0.count >= 2 }
    return lines.isEmpty ? nil : multiLineFeature(lines)
}

struct OtherMission: Equatable {
    var items: [MissionItem]
    var linkStartToHome: Bool
}

private let OTHER_VEHICLE_WAYPOINT = -1

func renderMission(
    _ style: MLNStyle,
    _ items: [MissionItem],
    _ linkStartToHome: Bool,
    selectedIndex: Int? = nil,
    others: [OtherMission] = [],
    landings: [LandingPattern] = []
) {
    let otherMarkers: [Feature] = others.flatMap { other in
        missionFeatures(other.items).shapes.map { feature in
            feature.attributes = feature.attributes.merging([WAYPOINT_ID_PROPERTY: OTHER_VEHICLE_WAYPOINT]) { _, mine in mine }
            return feature
        }
    }
    style.setGeoJson(MISSION_SOURCE, featureCollection(missionFeatures(items, selectedIndex: selectedIndex, landings: landings).shapes + otherMarkers))
    let paths = [missionPath(items, linkStartToHome)].compactMap { $0 } + others.compactMap { missionPath($0.items, $0.linkStartToHome) }
    style.setGeoJson(MISSION_PATH_SOURCE, featureCollection(paths))
}

let FENCE_SOURCE = "aircast-fence"
let FENCE_FILL_LAYER = "aircast-fence-fill"
let FENCE_LINE_LAYER = "aircast-fence-line"
let FIRMWARE_FENCE_SOURCE = "aircast-firmware-fence"
let FIRMWARE_FENCE_LAYER = "aircast-firmware-fence-line"
let RALLY_SOURCE = "aircast-rally"
let RALLY_LAYER = "aircast-rally-layer"
let RALLY_LABEL_LAYER = "aircast-rally-label-layer"

private func fenceColour() -> NSExpression {
    styleExpression(["case", ["get", KEEPS_IN_PROPERTY], KEEP_IN_COLOUR, KEEP_OUT_COLOUR])
}

let SHOT_SOURCE = "aircast-shots"
let SHOT_LAYER = "aircast-shots-dots"
let SHOT_COLOUR = "#FFFFFF"

func installShotLayer(_ style: MLNStyle) {
    if style.source(withIdentifier: SHOT_SOURCE) == nil {
        style.addSource(geoJsonSource(SHOT_SOURCE))
    }
    guard style.layer(withIdentifier: SHOT_LAYER) == nil, let source = style.source(withIdentifier: SHOT_SOURCE) else { return }
    let shots = MLNCircleStyleLayer(identifier: SHOT_LAYER, source: source)
    shots.circleRadius = styleConstant(3)
    shots.circleColor = styleConstant(mapColour(SHOT_COLOUR))
    shots.circleStrokeWidth = styleConstant(1)
    shots.circleStrokeColor = styleConstant(mapColour("#37474F"))
    style.addLayer(shots)
}

func shotFeatures(_ points: [TrackPoint]) -> FeatureCollection {
    featureCollection(points.map { pointFeature($0) })
}

func installFenceLayers(_ style: MLNStyle) {
    if style.source(withIdentifier: FENCE_SOURCE) == nil {
        let source = geoJsonSource(FENCE_SOURCE)
        style.addSource(source)
        let fill = MLNFillStyleLayer(identifier: FENCE_FILL_LAYER, source: source)
        fill.fillColor = fenceColour()
        fill.fillOpacity = styleConstant(0.15)
        style.addLayer(fill)
        let line = MLNLineStyleLayer(identifier: FENCE_LINE_LAYER, source: source)
        line.lineColor = fenceColour()
        line.lineWidth = styleConstant(2.5)
        style.addLayer(line)
    }

    if style.source(withIdentifier: FIRMWARE_FENCE_SOURCE) == nil {
        let source = geoJsonSource(FIRMWARE_FENCE_SOURCE)
        style.addSource(source)
        let line = MLNLineStyleLayer(identifier: FIRMWARE_FENCE_LAYER, source: source)
        line.lineColor = styleConstant(mapColour(FIRMWARE_FENCE_COLOUR))
        line.lineWidth = styleConstant(2)
        line.lineDashPattern = styleConstant([3, 3])
        style.addLayer(line)
    }

    if style.source(withIdentifier: GCS_SOURCE) == nil {
        let source = geoJsonSource(GCS_SOURCE)
        style.addSource(source)
        let dot = MLNCircleStyleLayer(identifier: GCS_LAYER, source: source)
        dot.circleColor = styleConstant(UIColor.white)
        dot.circleRadius = styleConstant(6)
        dot.circleStrokeColor = styleConstant(mapColour("#1976D2"))
        dot.circleStrokeWidth = styleConstant(3)
        style.addLayer(dot)
        let heading = MLNSymbolStyleLayer(identifier: GCS_HEADING_LAYER, source: source)
        heading.text = styleConstant("\u{25B2}")
        heading.textFontNames = styleConstant(NOTO_SANS)
        heading.textFontSize = styleConstant(12)
        heading.textColor = styleConstant(mapColour("#1976D2"))
        heading.textOpacity = styleConstant(0.85)
        heading.textRotation = styleGet(GCS_HEADING_PROPERTY)
        heading.textRotationAlignment = styleConstant("map")
        heading.textOffset = styleOffset(0, -1.1)
        heading.textAllowsOverlap = styleConstant(true)
        heading.textIgnoresPlacement = styleConstant(true)
        heading.predicate = NSPredicate(format: "%K != nil", GCS_HEADING_PROPERTY)
        style.addLayer(heading)
    }

    if style.source(withIdentifier: BREACH_SOURCE) == nil {
        let source = geoJsonSource(BREACH_SOURCE)
        style.addSource(source)
        let dot = MLNCircleStyleLayer(identifier: BREACH_LAYER, source: source)
        dot.circleColor = styleConstant(mapColour(KEEP_IN_COLOUR))
        dot.circleRadius = styleConstant(10)
        dot.circleStrokeColor = styleConstant(UIColor.black)
        dot.circleStrokeWidth = styleConstant(2)
        style.addLayer(dot)
        let label = MLNSymbolStyleLayer(identifier: BREACH_LABEL_LAYER, source: source)
        label.text = styleConstant("B")
        label.textFontSize = styleConstant(12)
        label.textColor = styleConstant(UIColor.black)
        label.textAllowsOverlap = styleConstant(true)
        label.textIgnoresPlacement = styleConstant(true)
        style.addLayer(label)
    }

    if style.source(withIdentifier: RALLY_SOURCE) == nil {
        let source = geoJsonSource(RALLY_SOURCE)
        style.addSource(source)
        let dot = MLNCircleStyleLayer(identifier: RALLY_LAYER, source: source)
        dot.circleColor = styleConstant(mapColour("#66BB6A"))
        dot.circleRadius = styleConstant(10)
        dot.circleStrokeColor = styleConstant(mapColour("#1B5E20"))
        dot.circleStrokeWidth = styleConstant(2)
        style.addLayer(dot)
        let label = MLNSymbolStyleLayer(identifier: RALLY_LABEL_LAYER, source: source)
        label.text = styleConstant("R")
        label.textFontSize = styleConstant(12)
        label.textColor = styleConstant(UIColor.black)
        label.textAllowsOverlap = styleConstant(true)
        label.textIgnoresPlacement = styleConstant(true)
        style.addLayer(label)
    }
}

let BREACH_SOURCE = "aircast-breach-return"
let BREACH_LAYER = "aircast-breach-return-layer"
let BREACH_LABEL_LAYER = "aircast-breach-return-label"

func breachFeatures(_ point: TrackPoint?) -> FeatureCollection {
    featureCollection([point].compactMap { $0 }.map { pointFeature($0) })
}

let GCS_SOURCE = "aircast-gcs"
let GCS_LAYER = "aircast-gcs-layer"

let GCS_HEADING_LAYER = "aircast-gcs-heading-layer"
let GCS_HEADING_PROPERTY = "heading"

func operatorFeatures(_ point: TrackPoint?, heading: Double = .nan) -> FeatureCollection {
    featureCollection([point].compactMap { $0 }.map { at in
        pointFeature(at, attributes: featureProperties((GCS_HEADING_PROPERTY, heading.isNaN ? nil : heading)))
    })
}

let CIRCLE_INDEX_PROPERTY = "circleIndex"
let KEEPS_IN_PROPERTY = "keepsIn"
let KEEP_IN_COLOUR = "#FF9500"
let KEEP_OUT_COLOUR = "#FF3B30"
let FIRMWARE_FENCE_COLOUR = "#AF52DE"

private func ringFeature(_ vertices: [TrackPoint], attributes: [String: Any] = [:]) -> Feature {
    polygonFeature(vertices.first == vertices.last ? vertices : vertices + [vertices[0]], attributes: attributes)
}

func fenceFeatures(_ polygons: [FencePolygon], circles: [FencePolygon] = []) -> FeatureCollection {
    featureCollection(
        polygons.map { ringFeature($0.vertices, attributes: [KEEPS_IN_PROPERTY: $0.inclusion]) }
            + circles.map { ringFeature($0.vertices, attributes: [CIRCLE_INDEX_PROPERTY: $0.index, KEEPS_IN_PROPERTY: $0.inclusion]) }
    )
}

let RALLY_INDEX_PROPERTY = "rallyIndex"

func rallyFeatures(_ points: [RallyPoint]) -> FeatureCollection {
    featureCollection(points.map { pointFeature(TrackPoint(latitude: $0.latitude, longitude: $0.longitude), attributes: [RALLY_INDEX_PROPERTY: $0.index]) })
}

func firmwareFenceFeatures(_ fence: FirmwareFence?) -> FeatureCollection {
    guard let fence, let centre = fence.centre else { return featureCollection([]) }
    return featureCollection([ringFeature(circleRing(centre, fence.radiusMetres))])
}

func renderFences(
    _ style: MLNStyle,
    _ polygons: [FencePolygon],
    _ rally: [RallyPoint],
    circles: [FencePolygon] = [],
    firmware: FirmwareFence? = nil,
    breach: TrackPoint? = nil
) {
    style.setGeoJson(BREACH_SOURCE, breachFeatures(breach))
    style.setGeoJson(FENCE_SOURCE, fenceFeatures(polygons, circles: circles))
    style.setGeoJson(RALLY_SOURCE, rallyFeatures(rally))
    style.setGeoJson(FIRMWARE_FENCE_SOURCE, firmwareFenceFeatures(firmware))
}

let FENCE_HANDLE_SOURCE = "aircast-fence-handles"
let FENCE_HANDLE_LAYER = "aircast-fence-handle-layer"

let POLYGON_INDEX_PROPERTY = "polygonIndex"
let VERTEX_INDEX_PROPERTY = "vertexIndex"
let HANDLE_KIND_PROPERTY = "handleKind"

let HANDLE_KIND_FENCE = "fence"
let HANDLE_KIND_SURVEY = "survey"
let HANDLE_KIND_CIRCLE = "circle"
let HANDLE_KIND_LANDING = "landing"
let HANDLE_KIND_FENCE_CENTRE = "fenceCentre"
let HANDLE_KIND_SURVEY_CENTRE = "surveyCentre"
let HANDLE_KIND_CIRCLE_RADIUS = "circleRadius"
let HANDLE_KIND_FENCE_CIRCLE_RADIUS = "fenceCircleRadius"
let HANDLE_KIND_LOITER_RADIUS = "loiterRadius"
let HANDLE_KIND_LOITER_ROTATION = "loiterRotation"

let SHAPE_PATH_PROPERTY = "shapePath"
let SPLIT_INVOKABLE_PROPERTY = "splitInvokable"
let MIDPOINT_SOURCE = "aircast-midpoints"
let MIDPOINT_LAYER = "aircast-midpoints-layer"

let LANDING_PLACE_APPROACH = 0
let LANDING_PLACE_TOUCHDOWN = 1

func installFenceHandleLayer(_ style: MLNStyle) {
    guard style.source(withIdentifier: FENCE_HANDLE_SOURCE) == nil else { return }
    let source = geoJsonSource(FENCE_HANDLE_SOURCE)
    style.addSource(source)
    let handles = MLNCircleStyleLayer(identifier: FENCE_HANDLE_LAYER, source: source)
    handles.circleColor = styleConstant(UIColor.white)
    handles.circleRadius = styleConstant(7)
    handles.circleStrokeColor = styleConstant(mapColour("#1565C0"))
    handles.circleStrokeWidth = styleConstant(3)
    handles.circleOpacity = tapOnlyHidden()
    handles.circleStrokeOpacity = tapOnlyHidden()
    style.addLayer(handles)
}

private func tapOnlyHidden() -> NSExpression {
    styleExpression(["match", ["get", HANDLE_KIND_PROPERTY], HANDLE_KIND_LOITER_ROTATION, 0, 1])
}

private func handleFeature(_ kind: String, _ owner: Int, _ vertex: Int, _ at: TrackPoint) -> Feature {
    pointFeature(at, attributes: [HANDLE_KIND_PROPERTY: kind, POLYGON_INDEX_PROPERTY: owner, VERTEX_INDEX_PROPERTY: vertex])
}

private func handleFeatures(_ kind: String, _ owner: Int, _ vertices: [TrackPoint]) -> [Feature] {
    vertices.enumerated().map { vertex, point in handleFeature(kind, owner, vertex, point) }
}

func vertexHandleFeatures(
    _ polygons: [FencePolygon],
    _ surveys: [Survey],
    circles: [FenceCircle] = [],
    landings: [LandingPattern] = []
) -> FeatureCollection {
    let fence = polygons.flatMap { handleFeatures(HANDLE_KIND_FENCE, $0.index, $0.vertices) }
    let survey = surveys.flatMap { handleFeatures(HANDLE_KIND_SURVEY, $0.index, $0.area) }
    let centres = circles.flatMap { handleFeatures(HANDLE_KIND_CIRCLE, $0.index, [$0.centre]) }
    let edges = circles.flatMap { handleFeatures(HANDLE_KIND_FENCE_CIRCLE_RADIUS, $0.index, [circleEdge($0)]) }
    let places = landings.flatMap(landingHandleFeatures)
    return featureCollection(fence + survey + centres + edges + places)
}

private func landingHandleFeatures(_ pattern: LandingPattern) -> [Feature] {
    [
        pattern.finalApproach.map { (LANDING_PLACE_APPROACH, $0) },
        pattern.landing.map { (LANDING_PLACE_TOUCHDOWN, $0) },
    ].compactMap { $0 }.map { place, at in handleFeature(HANDLE_KIND_LANDING, pattern.index, place, at) }
}

func renderVertexHandles(
    _ style: MLNStyle,
    _ polygons: [FencePolygon],
    _ surveys: [Survey],
    circles: [FenceCircle] = [],
    landings: [LandingPattern] = [],
    circled: Set<String> = [],
    loiterHandles: [Feature] = []
) {
    let cornered = polygons.map { polygon in circled.contains(fencePath(polygon.index)) ? withChanges(polygon) { $0.vertices = [] } : polygon }
    let cornerSurveys = surveys.map { survey in circled.contains(surveyPath(survey)) ? withChanges(survey) { $0.area = [] } : survey }
    style.setGeoJson(
        FENCE_HANDLE_SOURCE,
        featureCollection(
            vertexHandleFeatures(cornered, cornerSurveys, circles: circles, landings: landings).shapes
                + centreHandleFeatures(polygons, surveys)
                + radiusHandleFeatures(polygons, surveys, circled)
                + loiterHandles
        )
    )
}

let MINIMUM_CIRCLE_RADIUS_METRES = 0.1

func circleEdge(_ circle: FenceCircle) -> TrackPoint { pointAt(circle.centre, circle.radiusMetres, 90) }

func draggedCircleRadius(_ circle: FenceCircle, _ to: TrackPoint) -> Double {
    let shownPerMetre = circle.radiusMetres > 0 ? circle.radius / circle.radiusMetres : 1
    let wanted = metresBetween(circle.centre, to) * shownPerMetre
    return min(max(wanted, circle.radiusMinimum ?? MINIMUM_CIRCLE_RADIUS_METRES * shownPerMetre), circle.radiusMaximum ?? .greatestFiniteMagnitude)
}

func radiusHandleFeatures(_ polygons: [FencePolygon], _ surveys: [Survey], _ circled: Set<String>) -> [Feature] {
    func handle(_ vertices: [TrackPoint], _ owner: Int, _ fence: Bool) -> Feature? {
        polygonCentre(vertices).flatMap { centre in circleRadius(vertices).map { pointAt(centre, $0, 90) } }
            .map { handleFeature(HANDLE_KIND_CIRCLE_RADIUS, owner, fence ? 0 : 1, $0) }
    }
    return polygons.filter { circled.contains(fencePath($0.index)) }.compactMap { handle($0.vertices, $0.index, true) }
        + surveys.filter { circled.contains(surveyPath($0)) }.compactMap { handle($0.area, $0.index, false) }
}

private let loiterArrowAzimuths = [0.0, 180.0]

private func loiterItems(_ items: [MissionItem]) -> [MissionItem] {
    items.filter { $0.loiterRadius.isFinite && $0.loiterRadius != 0 && isPlottable($0.latitude, $0.longitude) }
}

func draggedLoiterRadius(_ item: MissionItem, _ to: TrackPoint) -> Double {
    let metres = metresBetween(TrackPoint(latitude: item.latitude, longitude: item.longitude), to)
    return item.loiterRadius >= 0 ? metres : -metres
}

func loiterRotationArrows(_ items: [MissionItem]) -> [TransectArrow] {
    loiterItems(items).flatMap { item in
        let centre = TrackPoint(latitude: item.latitude, longitude: item.longitude)
        return loiterArrowAzimuths.map { azimuth in
            TransectArrow(
                at: pointAt(centre, abs(item.loiterRadius), azimuth),
                bearing: (azimuth + (item.loiterRadius >= 0 ? 90 : 270)).truncatingRemainder(dividingBy: 360)
            )
        }
    }
}

func loiterHandleFeatures(_ items: [MissionItem], _ selected: Int?) -> [Feature] {
    let current = loiterItems(items).filter { $0.index == selected }
    let rotations = current.flatMap { item in
        loiterRotationArrows([item]).enumerated().map { vertex, arrow in handleFeature(HANDLE_KIND_LOITER_ROTATION, item.index, vertex, arrow.at) }
    }
    let radius = current.map { item in
        handleFeature(HANDLE_KIND_LOITER_RADIUS, item.index, 0, pointAt(TrackPoint(latitude: item.latitude, longitude: item.longitude), abs(item.loiterRadius), 90))
    }
    return rotations + radius
}

func centreHandleFeatures(_ polygons: [FencePolygon], _ surveys: [Survey]) -> [Feature] {
    polygons.compactMap { fence in polygonCentre(fence.vertices).map { handleFeatures(HANDLE_KIND_FENCE_CENTRE, fence.index, [$0]) } }.flatMap { $0 }
        + surveys.filter { $0.property != CORRIDOR_PROPERTY }
        .compactMap { area in polygonCentre(area.area).map { handleFeatures(HANDLE_KIND_SURVEY_CENTRE, area.index, [$0]) } }
        .flatMap { $0 }
}

let SURVEY_AREA_SOURCE = "aircast-survey-area"
let SURVEY_AREA_LAYER = "aircast-survey-area-layer"
let SURVEY_COLLISION = "collision"
let TERRAIN_COLLISION = "terrainCollision"
let SURVEY_TRANSECT_SOURCE = "aircast-survey-transects"
let SURVEY_TRANSECT_LAYER = "aircast-survey-transect-layer"
let LANDING_PATH_SOURCE = "aircast-landing-path"
let LANDING_PATH_LAYER = "aircast-landing-path-layer"
let LANDING_LOITER_SOURCE = "aircast-landing-loiter"
let LANDING_AREA_SOURCE = "aircast-landing-area"
let LANDING_AREA_LAYER = "aircast-landing-area-layer"
let LANDING_SHAPE_KIND = "kind"
let LANDING_LABEL_SOURCE = "aircast-landing-labels"
let LANDING_LABEL_LAYER = "aircast-landing-label-layer"
let LANDING_LABEL_TEXT = "text"
let EDGE_LABEL_SOURCE = "aircast-edge-labels"
let EDGE_LABEL_LAYER = "aircast-edge-label-layer"
let LANDING_LOITER_LAYER = "aircast-landing-loiter-layer"

let SURVEY_LINE_SOURCE = "aircast-survey-line"
let SURVEY_LINE_LAYER = "aircast-survey-line-layer"

func installMidpointLayer(_ style: MLNStyle) {
    guard style.source(withIdentifier: MIDPOINT_SOURCE) == nil else { return }
    let source = geoJsonSource(MIDPOINT_SOURCE)
    style.addSource(source)
    let midpoints = MLNCircleStyleLayer(identifier: MIDPOINT_LAYER, source: source)
    midpoints.circleColor = styleConstant(mapColour("#1565C0"))
    midpoints.circleRadius = styleConstant(5)
    midpoints.circleStrokeColor = styleConstant(UIColor.white)
    midpoints.circleStrokeWidth = styleConstant(2)
    midpoints.circleOpacity = styleConstant(0.85)
    style.addLayer(midpoints)
}

func midpointFeatures(_ shapes: [EditableShape?]) -> FeatureCollection {
    featureCollection(
        shapes.compactMap { $0 }.filter { !$0.splitInvokable.isBlank }.flatMap { shape in
            shape.midpoints.enumerated().map { segment, at in
                pointFeature(at, attributes: [SHAPE_PATH_PROPERTY: shape.path, SPLIT_INVOKABLE_PROPERTY: shape.splitInvokable, VERTEX_INDEX_PROPERTY: segment])
            }
        }
    )
}

let MISSION_SPLIT_PATH = "plan.missionController"
let MISSION_SPLIT_INVOKABLE = "insertSimpleMissionItem"

func legSplit(_ items: [MissionItem], _ selected: Int?) -> TrackPoint? {
    guard let current = items.first(where: { $0.index == selected }), current.routed,
          let previous = items.filter({ $0.index >= 1 && $0.index < current.index && $0.routed }).max(by: { $0.index < $1.index }) else { return nil }
    let from = previous.exit ?? TrackPoint(latitude: previous.latitude, longitude: previous.longitude)
    let to = TrackPoint(latitude: current.latitude, longitude: current.longitude)
    return pointAt(from, metresBetween(from, to) / 2, azimuthBetween(from, to))
}

func renderMidpoints(_ style: MLNStyle, _ polygons: [FencePolygon], _ surveys: [Survey], items: [MissionItem] = [], selected: Int? = nil) {
    let split = legSplit(items, selected).map { at in
        pointFeature(at, attributes: [SHAPE_PATH_PROPERTY: MISSION_SPLIT_PATH, SPLIT_INVOKABLE_PROPERTY: MISSION_SPLIT_INVOKABLE, VERTEX_INDEX_PROPERTY: selected ?? 0])
    }
    let shapes = midpointFeatures(polygons.map(\.editable) + surveys.map(\.editable)).shapes
    style.setGeoJson(MIDPOINT_SOURCE, featureCollection(shapes + [split].compactMap { $0 }))
}

private func edgeLabelLayer(_ identifier: String, _ source: MLNSource) -> MLNSymbolStyleLayer {
    let labels = MLNSymbolStyleLayer(identifier: identifier, source: source)
    labels.text = styleGet(LANDING_LABEL_TEXT)
    labels.textColor = styleConstant(UIColor.white)
    labels.textHaloColor = styleConstant(UIColor.black)
    labels.textHaloWidth = styleConstant(1.5)
    labels.textFontSize = styleConstant(12)
    labels.textAllowsOverlap = styleConstant(true)
    labels.textIgnoresPlacement = styleConstant(true)
    return labels
}

func installLandingLayers(_ style: MLNStyle) {
    if style.source(withIdentifier: EDGE_LABEL_SOURCE) == nil {
        let source = geoJsonSource(EDGE_LABEL_SOURCE)
        style.addSource(source)
        style.addLayer(edgeLabelLayer(EDGE_LABEL_LAYER, source))
    }
    if style.source(withIdentifier: LANDING_LABEL_SOURCE) == nil {
        let source = geoJsonSource(LANDING_LABEL_SOURCE)
        style.addSource(source)
        style.addLayer(edgeLabelLayer(LANDING_LABEL_LAYER, source))
    }
    if style.source(withIdentifier: LANDING_AREA_SOURCE) == nil {
        let source = geoJsonSource(LANDING_AREA_SOURCE)
        style.addSource(source)
        let area = MLNFillStyleLayer(identifier: LANDING_AREA_LAYER, source: source)
        area.fillColor = styleExpression([
            "case", ["get", TERRAIN_COLLISION], "#FF0000",
            ["match", ["get", LANDING_SHAPE_KIND], LANDING_AREA_KIND, "#00FF00", "#FFA500"],
        ])
        area.fillOpacity = styleConstant(0.5)
        area.fillOutlineColor = styleConstant(UIColor.black)
        style.addLayer(area)
    }
    if style.source(withIdentifier: LANDING_LOITER_SOURCE) == nil {
        let source = geoJsonSource(LANDING_LOITER_SOURCE)
        style.addSource(source)
        let loiter = MLNLineStyleLayer(identifier: LANDING_LOITER_LAYER, source: source)
        loiter.lineColor = styleExpression(["case", ["get", TERRAIN_COLLISION], "#FF0000", "#26C6DA"])
        loiter.lineWidth = styleConstant(2.5)
        style.addLayer(loiter)
    }

    if style.source(withIdentifier: LANDING_PATH_SOURCE) == nil {
        let source = geoJsonSource(LANDING_PATH_SOURCE)
        style.addSource(source)
        let path = MLNLineStyleLayer(identifier: LANDING_PATH_LAYER, source: source)
        path.lineColor = styleConstant(mapColour("#26C6DA"))
        path.lineWidth = styleConstant(3)
        path.lineDashPattern = styleConstant([3, 2])
        style.addLayer(path)
    }
}

func installSurveyLayers(_ style: MLNStyle) {
    if style.source(withIdentifier: SURVEY_AREA_SOURCE) == nil {
        let source = geoJsonSource(SURVEY_AREA_SOURCE)
        style.addSource(source)
        let area = MLNFillStyleLayer(identifier: SURVEY_AREA_LAYER, source: source)
        area.fillColor = styleExpression(["case", ["get", SURVEY_COLLISION], "#FF0000", "#AB47BC"])
        area.fillOpacity = styleConstant(0.18)
        area.fillOutlineColor = styleConstant(mapColour("#7B1FA2"))
        style.addLayer(area)
    }

    if style.source(withIdentifier: SURVEY_LINE_SOURCE) == nil {
        let source = geoJsonSource(SURVEY_LINE_SOURCE)
        style.addSource(source)
        let line = MLNLineStyleLayer(identifier: SURVEY_LINE_LAYER, source: source)
        line.lineColor = styleConstant(mapColour("#7B1FA2"))
        line.lineWidth = styleConstant(3)
        style.addLayer(line)
    }

    if style.source(withIdentifier: SURVEY_TRANSECT_SOURCE) == nil {
        let source = geoJsonSource(SURVEY_TRANSECT_SOURCE)
        style.addSource(source)
        let transects = MLNLineStyleLayer(identifier: SURVEY_TRANSECT_LAYER, source: source)
        transects.lineColor = styleConstant(mapColour("#E040FB"))
        transects.lineWidth = styleConstant(2.5)
        style.addLayer(transects)
    }
}

func shadedArea(_ survey: Survey) -> [TrackPoint] { survey.shape == SHAPE_AREA ? survey.area : survey.outline }

func surveyAreaFeatures(_ surveys: [Survey]) -> FeatureCollection {
    featureCollection(
        surveys.compactMap { survey in
            let area = shadedArea(survey)
            return area.count < 3 ? nil : ringFeature(area, attributes: [SURVEY_COLLISION: survey.collides])
        }
    )
}

func surveyLineFeatures(_ surveys: [Survey]) -> FeatureCollection {
    featureCollection(surveys.filter { $0.shape == SHAPE_LINE && $0.area.count >= 2 }.map { lineFeature($0.area) })
}

func flownRoute(_ survey: Survey) -> [TrackPoint] {
    survey.transects.count >= 2 ? survey.transects : survey.flightLoop.count >= 2 ? closedLoop(survey.flightLoop) : []
}

private func closedLoop(_ points: [TrackPoint]) -> [TrackPoint] {
    points.first == points.last ? points : points + [points[0]]
}

func surveyTransectFeatures(_ surveys: [Survey]) -> FeatureCollection {
    featureCollection(surveys.map(flownRoute).filter { $0.count >= 2 }.map { lineFeature($0) })
}

func renderSurveys(_ style: MLNStyle, _ surveys: [Survey]) {
    style.setGeoJson(SURVEY_AREA_SOURCE, surveyAreaFeatures(surveys))
    style.setGeoJson(SURVEY_LINE_SOURCE, surveyLineFeatures(surveys))
    style.setGeoJson(SURVEY_TRANSECT_SOURCE, surveyTransectFeatures(surveys))
}
