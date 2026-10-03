package one.aircast.mapspike

import org.maplibre.android.maps.Style
import org.maplibre.android.style.expressions.Expression
import org.maplibre.android.style.layers.CircleLayer
import org.maplibre.android.style.layers.FillLayer
import org.maplibre.android.style.layers.LineLayer
import org.maplibre.android.style.layers.Property
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.layers.SymbolLayer
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.LineString
import org.maplibre.geojson.MultiLineString
import org.maplibre.geojson.Point
import org.maplibre.geojson.Polygon

internal const val CROWDED_ITEMS = 40
internal const val CROWDED_RADIUS = 4.0
internal const val MARKER_RADIUS = 13.0
private const val CROWDED_STROKE = 0.5
private const val MARKER_STROKE = 2.0
private const val SELECTED_STROKE = 5.0
const val WAYPOINT_RADIUS_PROPERTY = "waypointRadius"
const val WAYPOINT_STROKE_PROPERTY = "waypointStroke"


const val MISSION_SOURCE = "aircast-mission"
const val MISSION_LAYER = "aircast-mission-layer"
const val MISSION_DOT_LAYER = "aircast-mission-dot-layer"
const val MISSION_PATH_SOURCE = "aircast-mission-path"
const val MISSION_PATH_LAYER = "aircast-mission-path-layer"
private const val COLLISION_LEG_SOURCE = "aircast-collision-legs"
private const val COLLISION_LEG_LAYER = "aircast-collision-leg-layer"
private const val COLLISION_COLOUR = "#FF0000"

fun collisionLegFeatures(legs: List<Pair<TrackPoint, TrackPoint>>): FeatureCollection =
    FeatureCollection.fromFeatures(
        legs.map { (from, to) -> Feature.fromGeometry(LineString.fromLngLats(listOf(Point.fromLngLat(from.longitude, from.latitude), Point.fromLngLat(to.longitude, to.latitude)))) },
    )

fun renderCollisionLegs(style: Style, legs: List<Pair<TrackPoint, TrackPoint>>) {
    (style.getSource(COLLISION_LEG_SOURCE) as? GeoJsonSource)?.setGeoJson(collisionLegFeatures(legs))
}

const val WAYPOINT_ID_PROPERTY = "waypointId"
internal const val WAYPOINT_LABEL_PROPERTY = "label"
internal const val WAYPOINT_SIDE_LABEL_PROPERTY = "sideLabel"
private const val MISSION_SIDE_LABEL_LAYER = "aircast-mission-side-label-layer"
const val WAYPOINT_COLOUR_PROPERTY = "waypointColour"
const val WAYPOINT_SELECTED_PROPERTY = "waypointSelected"

const val TAKEOFF_COLOUR = "#43A047"
const val LAND_COLOUR = "#E53935"
const val RETURN_COLOUR = "#5C6BC0"
const val LOITER_COLOUR = "#26A69A"
const val START_COLOUR = "#7E57C2"
const val WAYPOINT_COLOUR = "#FFB300"

const val MAV_CMD_NAV_LOITER_UNLIM = 17
const val MAV_CMD_NAV_LOITER_TURNS = 18
const val MAV_CMD_NAV_LOITER_TIME = 19
const val MAV_CMD_DO_ORBIT = 34

private val LOITER_COMMANDS = setOf(
    MAV_CMD_NAV_LOITER_UNLIM, MAV_CMD_NAV_LOITER_TURNS, MAV_CMD_NAV_LOITER_TIME, MAV_CMD_DO_ORBIT,
)

fun waypointColour(kind: String, commandId: Int): String = when {
    kind == "settings" -> START_COLOUR
    kind == "takeoff" -> TAKEOFF_COLOUR
    kind == "land" -> LAND_COLOUR
    commandId == MAV_CMD_NAV_RETURN_TO_LAUNCH -> RETURN_COLOUR
    commandId in LOITER_COMMANDS -> LOITER_COLOUR
    else -> WAYPOINT_COLOUR
}

fun installMissionLayers(style: Style) {
    if (style.getSource(MISSION_PATH_SOURCE) == null) {
        style.addSource(GeoJsonSource(MISSION_PATH_SOURCE))
        style.addLayer(
            LineLayer(MISSION_PATH_LAYER, MISSION_PATH_SOURCE).withProperties(
                PropertyFactory.lineColor("#FFB300"),
                PropertyFactory.lineWidth(3f),
                PropertyFactory.lineDasharray(arrayOf(2f, 1.5f)),
            ),
        )
    }
    if (style.getSource(COLLISION_LEG_SOURCE) == null) {
        style.addSource(GeoJsonSource(COLLISION_LEG_SOURCE))
        style.addLayer(
            LineLayer(COLLISION_LEG_LAYER, COLLISION_LEG_SOURCE).withProperties(
                PropertyFactory.lineColor(COLLISION_COLOUR),
                PropertyFactory.lineWidth(3f),
            ),
        )
    }

    if (style.getSource(MISSION_SOURCE) == null) {
        style.addSource(GeoJsonSource(MISSION_SOURCE))
        style.addLayer(
            CircleLayer(MISSION_DOT_LAYER, MISSION_SOURCE).withProperties(
                PropertyFactory.circleColor(Expression.get(WAYPOINT_COLOUR_PROPERTY)),
                PropertyFactory.circleRadius(Expression.get(WAYPOINT_RADIUS_PROPERTY)),
                PropertyFactory.circleStrokeColor(
                    Expression.switchCase(
                        Expression.get(WAYPOINT_SELECTED_PROPERTY), Expression.literal("#FFFFFF"),
                        Expression.literal("#37474F"),
                    ),
                ),
                PropertyFactory.circleStrokeWidth(Expression.get(WAYPOINT_STROKE_PROPERTY)),
            ),
        )
        style.addLayer(
            SymbolLayer(MISSION_LAYER, MISSION_SOURCE).withProperties(
                PropertyFactory.textField("{$WAYPOINT_LABEL_PROPERTY}"),
                PropertyFactory.textFont(arrayOf("Noto Sans Regular")),
                PropertyFactory.textSize(15f),
                PropertyFactory.textColor("#FFFFFF"),
                PropertyFactory.textHaloColor("#37474F"),
                PropertyFactory.textHaloWidth(2.5f),
                PropertyFactory.textAllowOverlap(false),
                PropertyFactory.textIgnorePlacement(false),
            ),
        )
        style.addLayer(
            SymbolLayer(MISSION_SIDE_LABEL_LAYER, MISSION_SOURCE).withProperties(
                PropertyFactory.textField("{$WAYPOINT_SIDE_LABEL_PROPERTY}"),
                PropertyFactory.textFont(arrayOf("Noto Sans Regular")),
                PropertyFactory.textSize(13f),
                PropertyFactory.textColor("#FFFFFF"),
                PropertyFactory.textHaloColor("#37474F"),
                PropertyFactory.textHaloWidth(2f),
                PropertyFactory.textAnchor(Property.TEXT_ANCHOR_LEFT),
                PropertyFactory.textOffset(arrayOf(1.2f, 0f)),
                PropertyFactory.textAllowOverlap(true),
            ),
        )
    }
}

fun crowded(itemCount: Int): Boolean = itemCount > CROWDED_ITEMS

fun waypointLabel(sequence: Int, crowded: Boolean, abbreviation: String = ""): String = when {
    crowded -> ""
    lettered(abbreviation) -> abbreviation.take(1)
    else -> sequence.toString()
}

fun sideLabel(crowded: Boolean, abbreviation: String): String =
    if (!crowded && lettered(abbreviation) && abbreviation.length > 1) abbreviation else ""

private fun lettered(abbreviation: String): Boolean = abbreviation.firstOrNull()?.let { it > 'A' && it < 'z' } == true

fun markerRadius(crowded: Boolean, selected: Boolean): Double =
    if (crowded && !selected) CROWDED_RADIUS else MARKER_RADIUS

fun markerStroke(crowded: Boolean, selected: Boolean): Double = when {
    selected -> SELECTED_STROKE
    crowded -> CROWDED_STROKE
    else -> MARKER_STROKE
}

private data class Marker(val item: MissionItem, val at: TrackPoint, val sequence: Int, val exit: Boolean, val side: String? = null)

fun exitMarkers(items: List<MissionItem>): List<Pair<MissionItem, TrackPoint>> =
    items.filter { it.complexPattern }.mapNotNull { item -> item.exit?.let { item to it } }

private fun landingMarkers(item: MissionItem, pattern: LandingPattern): List<Marker> = listOfNotNull(
    pattern.finalApproach?.let { Marker(item, it, item.sequence, exit = false, side = if (pattern.loiterToAltitude) "Loiter" else "Approach") },
    pattern.landing?.let { Marker(item, it, item.sequence + item.foldedCommands, exit = true, side = "Land") },
)

fun missionFeatures(items: List<MissionItem>, selectedIndex: Int? = null, landings: List<LandingPattern> = emptyList()): FeatureCollection {
    val crowded = crowded(items.size)
    val patterns = landings.associateBy { it.index }
    val plain = items.filter { it.index !in patterns }
    val markers = plain.map { item -> Marker(item, TrackPoint(item.latitude, item.longitude), item.sequence, exit = false) } +
        exitMarkers(plain).map { (item, exit) -> Marker(item, exit, item.sequence + item.foldedCommands, exit = true) } +
        items.flatMap { item -> patterns[item.index]?.let { landingMarkers(item, it) }.orEmpty() }
    val features = markers.map { (item, at, sequence, exit, side) ->
        val lettered = item.abbreviation.takeIf { !exit && !item.complexPattern }.orEmpty()
        Feature.fromGeometry(Point.fromLngLat(at.longitude, at.latitude)).apply {
            addNumberProperty(WAYPOINT_ID_PROPERTY, item.index)
            addStringProperty(WAYPOINT_LABEL_PROPERTY, waypointLabel(sequence, crowded, lettered))
            addStringProperty(WAYPOINT_SIDE_LABEL_PROPERTY, side?.takeUnless { crowded } ?: sideLabel(crowded, lettered))
            addNumberProperty(
                WAYPOINT_RADIUS_PROPERTY,
                markerRadius(crowded, item.index == selectedIndex),
            )
            addNumberProperty(
                WAYPOINT_STROKE_PROPERTY,
                markerStroke(crowded, item.index == selectedIndex),
            )
            addStringProperty(WAYPOINT_COLOUR_PROPERTY, waypointColour(item.kind, item.commandId))
            addBooleanProperty(WAYPOINT_SELECTED_PROPERTY, item.index == selectedIndex)
        }
    }
    return FeatureCollection.fromFeatures(features)
}

private const val LEG_ARROW_QUARTER = 3
private const val LEG_ARROW_SPACING = 5

private data class ArrowWalk(val arrows: List<Boolean>, val count: Int)

fun flownRoute(items: List<MissionItem>, linkStartToHome: Boolean): List<MissionItem> {
    val flown = (if (linkStartToHome) items else items.filterNot { it.index == 0 }).filter { it.routed && (it.index != 0 || it.placed) }
    val home = items.firstOrNull { it.index == 0 && it.closesRoute && it.placed }
    return if (home != null && flown.any { it.index != 0 }) flown + home.copy(exit = null) else flown
}

fun legArrows(items: List<MissionItem>, linkStartToHome: Boolean): List<TransectArrow> {
    val flown = flownRoute(items, linkStartToHome)
    val legs = flown.zipWithNext()
    val walk = legs.foldIndexed(ArrowWalk(emptyList(), 0)) { leg, walk, (from, to) ->
        when (to.index) {
            1 -> walk.copy(arrows = walk.arrows + false)
            else -> {
                val boundary = (leg == 0 && from.index == 0) || from.complexPattern || to.complexPattern
                val spaced = !boundary && walk.count > LEG_ARROW_SPACING
                walk.copy(arrows = walk.arrows + (boundary || spaced), count = if (spaced) 1 else walk.count + 1)
            }
        }
    }
    val marked = walk.arrows.mapIndexed { leg, arrow -> arrow || leg == legs.lastIndex }
    return legs.zip(marked).filter { (leg, arrow) -> arrow && !leg.second.legBroken }.map { (leg, _) ->
        val (from, to) = leg
        arrowOn(from.exit ?: TrackPoint(from.latitude, from.longitude), TrackPoint(to.latitude, to.longitude), LEG_ARROW_QUARTER)
    }
}

fun missionPath(items: List<MissionItem>, linkStartToHome: Boolean): Feature? {
    val runs = flownRoute(items, linkStartToHome).fold(emptyList<List<MissionItem>>()) { runs, item ->
        if (item.legBroken || runs.isEmpty()) runs + listOf(listOf(item)) else runs.dropLast(1) + listOf(runs.last() + item)
    }
    val lines = runs.map { run ->
        run.flatMap { item ->
            listOfNotNull(
                Point.fromLngLat(item.longitude, item.latitude),
                item.exit?.let { Point.fromLngLat(it.longitude, it.latitude) },
            )
        }
    }.filter { it.size >= 2 }
    return lines.takeIf { it.isNotEmpty() }?.let { Feature.fromGeometry(MultiLineString.fromLngLats(it)) }
}

data class OtherMission(val items: List<MissionItem>, val linkStartToHome: Boolean)

private const val OTHER_VEHICLE_WAYPOINT = -1

fun renderMission(
    style: Style,
    items: List<MissionItem>,
    linkStartToHome: Boolean,
    selectedIndex: Int? = null,
    others: List<OtherMission> = emptyList(),
    landings: List<LandingPattern> = emptyList(),
) {
    val otherMarkers = others.flatMap { other ->
        missionFeatures(other.items).features().orEmpty().onEach { it.addNumberProperty(WAYPOINT_ID_PROPERTY, OTHER_VEHICLE_WAYPOINT) }
    }
    (style.getSource(MISSION_SOURCE) as? GeoJsonSource)
        ?.setGeoJson(FeatureCollection.fromFeatures(missionFeatures(items, selectedIndex, landings).features().orEmpty() + otherMarkers))

    val paths = listOfNotNull(missionPath(items, linkStartToHome)) + others.mapNotNull { missionPath(it.items, it.linkStartToHome) }
    (style.getSource(MISSION_PATH_SOURCE) as? GeoJsonSource)?.setGeoJson(FeatureCollection.fromFeatures(paths))
}

const val FENCE_SOURCE = "aircast-fence"
const val FENCE_FILL_LAYER = "aircast-fence-fill"
const val FENCE_LINE_LAYER = "aircast-fence-line"
const val FIRMWARE_FENCE_SOURCE = "aircast-firmware-fence"
const val FIRMWARE_FENCE_LAYER = "aircast-firmware-fence-line"
const val RALLY_SOURCE = "aircast-rally"
const val RALLY_LAYER = "aircast-rally-layer"
const val RALLY_LABEL_LAYER = "aircast-rally-label-layer"

private fun fenceColour(): Expression = Expression.switchCase(
    Expression.get(KEEPS_IN_PROPERTY), Expression.literal(KEEP_IN_COLOUR),
    Expression.literal(KEEP_OUT_COLOUR),
)

const val SHOT_SOURCE = "aircast-shots"
const val SHOT_LAYER = "aircast-shots-dots"
const val SHOT_COLOUR = "#FFFFFF"

fun installShotLayer(style: Style) {
    if (style.getSource(SHOT_SOURCE) == null) {
        style.addSource(GeoJsonSource(SHOT_SOURCE))
    }
    if (style.getLayer(SHOT_LAYER) == null) {
        style.addLayer(
            CircleLayer(SHOT_LAYER, SHOT_SOURCE).withProperties(
                PropertyFactory.circleRadius(3f),
                PropertyFactory.circleColor(SHOT_COLOUR),
                PropertyFactory.circleStrokeWidth(1f),
                PropertyFactory.circleStrokeColor("#37474F"),
            ),
        )
    }
}

fun shotFeatures(points: List<TrackPoint>): FeatureCollection =
    FeatureCollection.fromFeatures(
        points.map { Feature.fromGeometry(Point.fromLngLat(it.longitude, it.latitude)) },
    )

fun installFenceLayers(style: Style) {
    if (style.getSource(FENCE_SOURCE) == null) {
        style.addSource(GeoJsonSource(FENCE_SOURCE))
        style.addLayer(
            FillLayer(FENCE_FILL_LAYER, FENCE_SOURCE).withProperties(
                PropertyFactory.fillColor(fenceColour()),
                PropertyFactory.fillOpacity(0.15f),
            ),
        )
        style.addLayer(
            LineLayer(FENCE_LINE_LAYER, FENCE_SOURCE).withProperties(
                PropertyFactory.lineColor(fenceColour()),
                PropertyFactory.lineWidth(2.5f),
            ),
        )
    }

    if (style.getSource(FIRMWARE_FENCE_SOURCE) == null) {
        style.addSource(GeoJsonSource(FIRMWARE_FENCE_SOURCE))
        style.addLayer(
            LineLayer(FIRMWARE_FENCE_LAYER, FIRMWARE_FENCE_SOURCE).withProperties(
                PropertyFactory.lineColor(FIRMWARE_FENCE_COLOUR),
                PropertyFactory.lineWidth(2.0f),
                PropertyFactory.lineDasharray(arrayOf(3.0f, 3.0f)),
            ),
        )
    }

    if (style.getSource(GCS_SOURCE) == null) {
        style.addSource(GeoJsonSource(GCS_SOURCE))
        style.addLayer(
            CircleLayer(GCS_LAYER, GCS_SOURCE).withProperties(
                PropertyFactory.circleColor("#FFFFFF"),
                PropertyFactory.circleRadius(6f),
                PropertyFactory.circleStrokeColor("#1976D2"),
                PropertyFactory.circleStrokeWidth(3f),
            ),
        )
        style.addLayer(
            SymbolLayer(GCS_HEADING_LAYER, GCS_SOURCE).withProperties(
                PropertyFactory.textField("\u25B2"),
                PropertyFactory.textFont(arrayOf("Noto Sans Regular")),
                PropertyFactory.textSize(12f),
                PropertyFactory.textColor("#1976D2"),
                PropertyFactory.textOpacity(0.85f),
                PropertyFactory.textRotate(Expression.get(GCS_HEADING_PROPERTY)),
                PropertyFactory.textRotationAlignment(Property.TEXT_ROTATION_ALIGNMENT_MAP),
                PropertyFactory.textOffset(arrayOf(0f, -1.1f)),
                PropertyFactory.textAllowOverlap(true),
                PropertyFactory.textIgnorePlacement(true),
            ).withFilter(Expression.has(GCS_HEADING_PROPERTY)),
        )
    }

    if (style.getSource(BREACH_SOURCE) == null) {
        style.addSource(GeoJsonSource(BREACH_SOURCE))
        style.addLayer(
            CircleLayer(BREACH_LAYER, BREACH_SOURCE).withProperties(
                PropertyFactory.circleColor(KEEP_IN_COLOUR),
                PropertyFactory.circleRadius(10f),
                PropertyFactory.circleStrokeColor("#000000"),
                PropertyFactory.circleStrokeWidth(2f),
            ),
        )
        style.addLayer(
            SymbolLayer(BREACH_LABEL_LAYER, BREACH_SOURCE).withProperties(
                PropertyFactory.textField("B"),
                PropertyFactory.textSize(12f),
                PropertyFactory.textColor("#000000"),
                PropertyFactory.textAllowOverlap(true),
                PropertyFactory.textIgnorePlacement(true),
            ),
        )
    }

    if (style.getSource(RALLY_SOURCE) == null) {
        style.addSource(GeoJsonSource(RALLY_SOURCE))
        style.addLayer(
            CircleLayer(RALLY_LAYER, RALLY_SOURCE).withProperties(
                PropertyFactory.circleColor("#66BB6A"),
                PropertyFactory.circleRadius(10f),
                PropertyFactory.circleStrokeColor("#1B5E20"),
                PropertyFactory.circleStrokeWidth(2f),
            ),
        )
        style.addLayer(
            SymbolLayer(RALLY_LABEL_LAYER, RALLY_SOURCE).withProperties(
                PropertyFactory.textField("R"),
                PropertyFactory.textSize(12f),
                PropertyFactory.textColor("#000000"),
                PropertyFactory.textAllowOverlap(true),
                PropertyFactory.textIgnorePlacement(true),
            ),
        )
    }
}

const val BREACH_SOURCE = "aircast-breach-return"
const val BREACH_LAYER = "aircast-breach-return-layer"
const val BREACH_LABEL_LAYER = "aircast-breach-return-label"

fun breachFeatures(point: TrackPoint?): FeatureCollection =
    FeatureCollection.fromFeatures(listOfNotNull(point).map { Feature.fromGeometry(Point.fromLngLat(it.longitude, it.latitude)) })

const val GCS_SOURCE = "aircast-gcs"
const val GCS_LAYER = "aircast-gcs-layer"

const val GCS_HEADING_LAYER = "aircast-gcs-heading-layer"
const val GCS_HEADING_PROPERTY = "heading"

fun operatorFeatures(point: TrackPoint?, heading: Double = Double.NaN): FeatureCollection =
    FeatureCollection.fromFeatures(
        listOfNotNull(point).map { at ->
            Feature.fromGeometry(Point.fromLngLat(at.longitude, at.latitude)).also { feature ->
                if (!heading.isNaN()) feature.addNumberProperty(GCS_HEADING_PROPERTY, heading)
            }
        },
    )

const val CIRCLE_INDEX_PROPERTY = "circleIndex"
const val KEEPS_IN_PROPERTY = "keepsIn"
const val KEEP_IN_COLOUR = "#FF9500"
const val KEEP_OUT_COLOUR = "#FF3B30"
const val FIRMWARE_FENCE_COLOUR = "#AF52DE"

private fun ringFeature(vertices: List<TrackPoint>): Feature {
    val ring = vertices.map { Point.fromLngLat(it.longitude, it.latitude) }
    val closed = if (ring.first() == ring.last()) ring else ring + ring.first()
    return Feature.fromGeometry(Polygon.fromLngLats(listOf(closed)))
}

fun fenceFeatures(
    polygons: List<FencePolygon>,
    circles: List<FencePolygon> = emptyList(),
): FeatureCollection {
    val polygonFeatures = polygons.map { polygon ->
        ringFeature(polygon.vertices).apply { addBooleanProperty(KEEPS_IN_PROPERTY, polygon.inclusion) }
    }
    val circleFeatures = circles.map { circle ->
        ringFeature(circle.vertices).apply {
            addNumberProperty(CIRCLE_INDEX_PROPERTY, circle.index)
            addBooleanProperty(KEEPS_IN_PROPERTY, circle.inclusion)
        }
    }
    return FeatureCollection.fromFeatures(polygonFeatures + circleFeatures)
}

const val RALLY_INDEX_PROPERTY = "rallyIndex"

fun rallyFeatures(points: List<RallyPoint>): FeatureCollection =
    FeatureCollection.fromFeatures(
        points.map { point ->
            Feature.fromGeometry(Point.fromLngLat(point.longitude, point.latitude)).apply {
                addNumberProperty(RALLY_INDEX_PROPERTY, point.index)
            }
        },
    )

fun firmwareFenceFeatures(fence: FirmwareFence?): FeatureCollection {
    val centre = fence?.centre ?: return FeatureCollection.fromFeatures(emptyList())
    return FeatureCollection.fromFeatures(listOf(ringFeature(circleRing(centre, fence.radiusMetres))))
}

fun renderFences(
    style: Style,
    polygons: List<FencePolygon>,
    rally: List<RallyPoint>,
    circles: List<FencePolygon> = emptyList(),
    firmware: FirmwareFence? = null,
    breach: TrackPoint? = null,
) {
    (style.getSource(BREACH_SOURCE) as? GeoJsonSource)?.setGeoJson(breachFeatures(breach))
    (style.getSource(FENCE_SOURCE) as? GeoJsonSource)?.setGeoJson(fenceFeatures(polygons, circles))
    (style.getSource(RALLY_SOURCE) as? GeoJsonSource)?.setGeoJson(rallyFeatures(rally))
    (style.getSource(FIRMWARE_FENCE_SOURCE) as? GeoJsonSource)?.setGeoJson(firmwareFenceFeatures(firmware))
}

const val FENCE_HANDLE_SOURCE = "aircast-fence-handles"
const val FENCE_HANDLE_LAYER = "aircast-fence-handle-layer"

const val POLYGON_INDEX_PROPERTY = "polygonIndex"
const val VERTEX_INDEX_PROPERTY = "vertexIndex"
const val HANDLE_KIND_PROPERTY = "handleKind"

const val HANDLE_KIND_FENCE = "fence"
const val HANDLE_KIND_SURVEY = "survey"
const val HANDLE_KIND_CIRCLE = "circle"
const val HANDLE_KIND_LANDING = "landing"
const val HANDLE_KIND_FENCE_CENTRE = "fenceCentre"
const val HANDLE_KIND_SURVEY_CENTRE = "surveyCentre"
const val HANDLE_KIND_CIRCLE_RADIUS = "circleRadius"
const val HANDLE_KIND_FENCE_CIRCLE_RADIUS = "fenceCircleRadius"
const val HANDLE_KIND_LOITER_RADIUS = "loiterRadius"
const val HANDLE_KIND_LOITER_ROTATION = "loiterRotation"

const val SHAPE_PATH_PROPERTY = "shapePath"
const val SPLIT_INVOKABLE_PROPERTY = "splitInvokable"
const val MIDPOINT_SOURCE = "aircast-midpoints"
const val MIDPOINT_LAYER = "aircast-midpoints-layer"

const val LANDING_PLACE_APPROACH = 0
const val LANDING_PLACE_TOUCHDOWN = 1

fun installFenceHandleLayer(style: Style) {
    if (style.getSource(FENCE_HANDLE_SOURCE) != null) {
        return
    }
    style.addSource(GeoJsonSource(FENCE_HANDLE_SOURCE))
    style.addLayer(
        CircleLayer(FENCE_HANDLE_LAYER, FENCE_HANDLE_SOURCE).withProperties(
            PropertyFactory.circleColor("#FFFFFF"),
            PropertyFactory.circleRadius(7f),
            PropertyFactory.circleStrokeColor("#1565C0"),
            PropertyFactory.circleStrokeWidth(3f),
            PropertyFactory.circleOpacity(tapOnlyHidden),
            PropertyFactory.circleStrokeOpacity(tapOnlyHidden),
        ),
    )
}

private val tapOnlyHidden: Expression =
    Expression.match(Expression.get(HANDLE_KIND_PROPERTY), Expression.literal(1f), Expression.stop(HANDLE_KIND_LOITER_ROTATION, 0f))

private fun handleFeatures(kind: String, owner: Int, vertices: List<TrackPoint>) =
    vertices.mapIndexed { vertex, point ->
        Feature.fromGeometry(Point.fromLngLat(point.longitude, point.latitude)).apply {
            addStringProperty(HANDLE_KIND_PROPERTY, kind)
            addNumberProperty(POLYGON_INDEX_PROPERTY, owner)
            addNumberProperty(VERTEX_INDEX_PROPERTY, vertex)
        }
    }

fun vertexHandleFeatures(
    polygons: List<FencePolygon>,
    surveys: List<Survey>,
    circles: List<FenceCircle> = emptyList(),
    landings: List<LandingPattern> = emptyList(),
): FeatureCollection {
    val fence = polygons.flatMap { handleFeatures(HANDLE_KIND_FENCE, it.index, it.vertices) }
    val survey = surveys.flatMap { handleFeatures(HANDLE_KIND_SURVEY, it.index, it.area) }
    val centres = circles.flatMap { handleFeatures(HANDLE_KIND_CIRCLE, it.index, listOf(it.centre)) }
    val edges = circles.flatMap { handleFeatures(HANDLE_KIND_FENCE_CIRCLE_RADIUS, it.index, listOf(circleEdge(it))) }
    val places = landings.flatMap { landingHandleFeatures(it) }
    return FeatureCollection.fromFeatures(fence + survey + centres + edges + places)
}

private fun landingHandleFeatures(pattern: LandingPattern) = listOfNotNull(
    pattern.finalApproach?.let { LANDING_PLACE_APPROACH to it },
    pattern.landing?.let { LANDING_PLACE_TOUCHDOWN to it },
).map { (place, at) ->
    Feature.fromGeometry(Point.fromLngLat(at.longitude, at.latitude)).apply {
        addStringProperty(HANDLE_KIND_PROPERTY, HANDLE_KIND_LANDING)
        addNumberProperty(POLYGON_INDEX_PROPERTY, pattern.index)
        addNumberProperty(VERTEX_INDEX_PROPERTY, place)
    }
}

fun renderVertexHandles(
    style: Style,
    polygons: List<FencePolygon>,
    surveys: List<Survey>,
    circles: List<FenceCircle> = emptyList(),
    landings: List<LandingPattern> = emptyList(),
    circled: Set<String> = emptySet(),
    loiterHandles: List<Feature> = emptyList(),
) {
    val cornered = polygons.map { if (fencePath(it.index) in circled) it.copy(vertices = emptyList()) else it }
    val cornerSurveys = surveys.map { if (surveyPath(it) in circled) it.copy(area = emptyList()) else it }
    (style.getSource(FENCE_HANDLE_SOURCE) as? GeoJsonSource)
        ?.setGeoJson(FeatureCollection.fromFeatures(vertexHandleFeatures(cornered, cornerSurveys, circles, landings).features().orEmpty() + centreHandleFeatures(polygons, surveys) + radiusHandleFeatures(polygons, surveys, circled) + loiterHandles))
}

const val MINIMUM_CIRCLE_RADIUS_METRES = 0.1

fun circleEdge(circle: FenceCircle): TrackPoint = pointAt(circle.centre, circle.radiusMetres, 90.0)

fun draggedCircleRadius(circle: FenceCircle, to: TrackPoint): Double {
    val shownPerMetre = if (circle.radiusMetres > 0.0) circle.radius / circle.radiusMetres else 1.0
    val wanted = metresBetween(circle.centre, to) * shownPerMetre
    return wanted.coerceAtLeast(circle.radiusMinimum ?: (MINIMUM_CIRCLE_RADIUS_METRES * shownPerMetre)).coerceAtMost(circle.radiusMaximum ?: Double.MAX_VALUE)
}

fun radiusHandleFeatures(polygons: List<FencePolygon>, surveys: List<Survey>, circled: Set<String>): List<Feature> {
    fun handle(vertices: List<TrackPoint>, owner: Int, fence: Boolean) =
        polygonCentre(vertices)?.let { centre -> circleRadius(vertices)?.let { radius -> pointAt(centre, radius, 90.0) } }?.let { at ->
            Feature.fromGeometry(Point.fromLngLat(at.longitude, at.latitude)).apply {
                addStringProperty(HANDLE_KIND_PROPERTY, HANDLE_KIND_CIRCLE_RADIUS)
                addNumberProperty(POLYGON_INDEX_PROPERTY, owner)
                addNumberProperty(VERTEX_INDEX_PROPERTY, if (fence) 0 else 1)
            }
        }
    return polygons.filter { fencePath(it.index) in circled }.mapNotNull { handle(it.vertices, it.index, true) } +
        surveys.filter { surveyPath(it) in circled }.mapNotNull { handle(it.area, it.index, false) }
}

private val loiterArrowAzimuths = listOf(0.0, 180.0)

private fun loiterItems(items: List<MissionItem>) =
    items.filter { it.loiterRadius.isFinite() && it.loiterRadius != 0.0 && isPlottable(it.latitude, it.longitude) }

fun draggedLoiterRadius(item: MissionItem, to: TrackPoint): Double =
    metresBetween(TrackPoint(item.latitude, item.longitude), to).let { if (item.loiterRadius >= 0) it else -it }

fun loiterRotationArrows(items: List<MissionItem>): List<TransectArrow> =
    loiterItems(items).flatMap { item ->
        val centre = TrackPoint(item.latitude, item.longitude)
        loiterArrowAzimuths.map { azimuth ->
            TransectArrow(pointAt(centre, kotlin.math.abs(item.loiterRadius), azimuth), (azimuth + if (item.loiterRadius >= 0) 90.0 else 270.0) % 360.0)
        }
    }

fun loiterHandleFeatures(items: List<MissionItem>, selected: Int?): List<Feature> {
    fun handle(at: TrackPoint, kind: String, owner: Int, vertex: Int) =
        Feature.fromGeometry(Point.fromLngLat(at.longitude, at.latitude)).apply {
            addStringProperty(HANDLE_KIND_PROPERTY, kind)
            addNumberProperty(POLYGON_INDEX_PROPERTY, owner)
            addNumberProperty(VERTEX_INDEX_PROPERTY, vertex)
        }
    val rotations = loiterItems(items).filter { it.index == selected }.flatMap { item ->
        loiterRotationArrows(listOf(item)).mapIndexed { vertex, arrow -> handle(arrow.at, HANDLE_KIND_LOITER_ROTATION, item.index, vertex) }
    }
    val radius = loiterItems(items).filter { it.index == selected }.map { item ->
        handle(pointAt(TrackPoint(item.latitude, item.longitude), kotlin.math.abs(item.loiterRadius), 90.0), HANDLE_KIND_LOITER_RADIUS, item.index, 0)
    }
    return rotations + radius
}

fun centreHandleFeatures(polygons: List<FencePolygon>, surveys: List<Survey>): List<Feature> =
    polygons.mapNotNull { fence -> polygonCentre(fence.vertices)?.let { handleFeatures(HANDLE_KIND_FENCE_CENTRE, fence.index, listOf(it)) } }.flatten() +
        surveys.filter { it.property != CORRIDOR_PROPERTY }.mapNotNull { area -> polygonCentre(area.area)?.let { handleFeatures(HANDLE_KIND_SURVEY_CENTRE, area.index, listOf(it)) } }.flatten()

const val SURVEY_AREA_SOURCE = "aircast-survey-area"
const val SURVEY_AREA_LAYER = "aircast-survey-area-layer"
const val SURVEY_COLLISION = "collision"
const val TERRAIN_COLLISION = "terrainCollision"
const val SURVEY_TRANSECT_SOURCE = "aircast-survey-transects"
const val SURVEY_TRANSECT_LAYER = "aircast-survey-transect-layer"
const val LANDING_PATH_SOURCE = "aircast-landing-path"
const val LANDING_PATH_LAYER = "aircast-landing-path-layer"
const val LANDING_LOITER_SOURCE = "aircast-landing-loiter"
const val LANDING_AREA_SOURCE = "aircast-landing-area"
const val LANDING_AREA_LAYER = "aircast-landing-area-layer"
const val LANDING_SHAPE_KIND = "kind"
const val LANDING_LABEL_SOURCE = "aircast-landing-labels"
const val LANDING_LABEL_LAYER = "aircast-landing-label-layer"
const val LANDING_LABEL_TEXT = "text"
const val EDGE_LABEL_SOURCE = "aircast-edge-labels"
const val EDGE_LABEL_LAYER = "aircast-edge-label-layer"
const val LANDING_LOITER_LAYER = "aircast-landing-loiter-layer"

const val SURVEY_LINE_SOURCE = "aircast-survey-line"
const val SURVEY_LINE_LAYER = "aircast-survey-line-layer"

fun installMidpointLayer(style: Style) {
    if (style.getSource(MIDPOINT_SOURCE) != null) {
        return
    }
    style.addSource(GeoJsonSource(MIDPOINT_SOURCE))
    style.addLayer(
        CircleLayer(MIDPOINT_LAYER, MIDPOINT_SOURCE).withProperties(
            PropertyFactory.circleColor("#1565C0"),
            PropertyFactory.circleRadius(5f),
            PropertyFactory.circleStrokeColor("#FFFFFF"),
            PropertyFactory.circleStrokeWidth(2f),
            PropertyFactory.circleOpacity(0.85f),
        ),
    )
}

fun midpointFeatures(shapes: List<EditableShape?>): FeatureCollection =
    FeatureCollection.fromFeatures(
        shapes.filterNotNull().filter { it.splitInvokable.isNotBlank() }.flatMap { shape ->
            shape.midpoints.mapIndexed { segment, at ->
                Feature.fromGeometry(Point.fromLngLat(at.longitude, at.latitude)).apply {
                    addStringProperty(SHAPE_PATH_PROPERTY, shape.path)
                    addStringProperty(SPLIT_INVOKABLE_PROPERTY, shape.splitInvokable)
                    addNumberProperty(VERTEX_INDEX_PROPERTY, segment)
                }
            }
        },
    )

const val MISSION_SPLIT_PATH = "plan.missionController"
const val MISSION_SPLIT_INVOKABLE = "insertSimpleMissionItem"

fun legSplit(items: List<MissionItem>, selected: Int?): TrackPoint? {
    val current = items.firstOrNull { it.index == selected }?.takeIf { it.routed } ?: return null
    val previous = items.filter { it.index in 1 until current.index && it.routed }.maxByOrNull { it.index } ?: return null
    val from = previous.exit ?: TrackPoint(previous.latitude, previous.longitude)
    val to = TrackPoint(current.latitude, current.longitude)
    return pointAt(from, metresBetween(from, to) / 2, azimuthBetween(from, to))
}

fun renderMidpoints(style: Style, polygons: List<FencePolygon>, surveys: List<Survey>, items: List<MissionItem> = emptyList(), selected: Int? = null) {
    val split = legSplit(items, selected)?.let { at ->
        Feature.fromGeometry(Point.fromLngLat(at.longitude, at.latitude)).apply {
            addStringProperty(SHAPE_PATH_PROPERTY, MISSION_SPLIT_PATH)
            addStringProperty(SPLIT_INVOKABLE_PROPERTY, MISSION_SPLIT_INVOKABLE)
            addNumberProperty(VERTEX_INDEX_PROPERTY, selected ?: 0)
        }
    }
    val shapes = midpointFeatures(polygons.map { it.editable } + surveys.map { it.editable }).features().orEmpty()
    (style.getSource(MIDPOINT_SOURCE) as? GeoJsonSource)?.setGeoJson(FeatureCollection.fromFeatures(shapes + listOfNotNull(split)))
}

fun installLandingLayers(style: Style) {
    if (style.getSource(EDGE_LABEL_SOURCE) == null) {
        style.addSource(GeoJsonSource(EDGE_LABEL_SOURCE))
        style.addLayer(
            SymbolLayer(EDGE_LABEL_LAYER, EDGE_LABEL_SOURCE).withProperties(
                PropertyFactory.textField(Expression.get(LANDING_LABEL_TEXT)),
                PropertyFactory.textColor(android.graphics.Color.WHITE),
                PropertyFactory.textHaloColor(android.graphics.Color.BLACK),
                PropertyFactory.textHaloWidth(1.5f),
                PropertyFactory.textSize(12f),
                PropertyFactory.textAllowOverlap(true),
                PropertyFactory.textIgnorePlacement(true),
            ),
        )
    }
    if (style.getSource(LANDING_LABEL_SOURCE) == null) {
        style.addSource(GeoJsonSource(LANDING_LABEL_SOURCE))
        style.addLayer(
            SymbolLayer(LANDING_LABEL_LAYER, LANDING_LABEL_SOURCE).withProperties(
                PropertyFactory.textField(Expression.get(LANDING_LABEL_TEXT)),
                PropertyFactory.textColor(android.graphics.Color.WHITE),
                PropertyFactory.textHaloColor(android.graphics.Color.BLACK),
                PropertyFactory.textHaloWidth(1.5f),
                PropertyFactory.textSize(12f),
                PropertyFactory.textAllowOverlap(true),
                PropertyFactory.textIgnorePlacement(true),
            ),
        )
    }
    if (style.getSource(LANDING_AREA_SOURCE) == null) {
        style.addSource(GeoJsonSource(LANDING_AREA_SOURCE))
        style.addLayer(
            FillLayer(LANDING_AREA_LAYER, LANDING_AREA_SOURCE).withProperties(
                PropertyFactory.fillColor(Expression.switchCase(Expression.get(TERRAIN_COLLISION), Expression.color(android.graphics.Color.RED), Expression.match(Expression.get(LANDING_SHAPE_KIND), Expression.literal(LANDING_AREA_KIND), Expression.color(android.graphics.Color.GREEN), Expression.color(android.graphics.Color.rgb(255, 165, 0))))),
                PropertyFactory.fillOpacity(0.5f),
                PropertyFactory.fillOutlineColor(android.graphics.Color.BLACK),
            ),
        )
    }
    if (style.getSource(LANDING_LOITER_SOURCE) == null) {
        style.addSource(GeoJsonSource(LANDING_LOITER_SOURCE))
        style.addLayer(
            LineLayer(LANDING_LOITER_LAYER, LANDING_LOITER_SOURCE).withProperties(
                PropertyFactory.lineColor(Expression.switchCase(Expression.get(TERRAIN_COLLISION), Expression.color(android.graphics.Color.RED), Expression.color(android.graphics.Color.parseColor("#26C6DA")))),
                PropertyFactory.lineWidth(2.5f),
            ),
        )
    }

    if (style.getSource(LANDING_PATH_SOURCE) == null) {
        style.addSource(GeoJsonSource(LANDING_PATH_SOURCE))
        style.addLayer(
            LineLayer(LANDING_PATH_LAYER, LANDING_PATH_SOURCE).withProperties(
                PropertyFactory.lineColor("#26C6DA"),
                PropertyFactory.lineWidth(3f),
                PropertyFactory.lineDasharray(arrayOf(3f, 2f)),
            ),
        )
    }
}

fun installSurveyLayers(style: Style) {
    if (style.getSource(SURVEY_AREA_SOURCE) == null) {
        style.addSource(GeoJsonSource(SURVEY_AREA_SOURCE))
        style.addLayer(
            FillLayer(SURVEY_AREA_LAYER, SURVEY_AREA_SOURCE).withProperties(
                PropertyFactory.fillColor(Expression.switchCase(Expression.get(SURVEY_COLLISION), Expression.color(android.graphics.Color.RED), Expression.color(android.graphics.Color.parseColor("#AB47BC")))),
                PropertyFactory.fillOpacity(0.18f),
                PropertyFactory.fillOutlineColor("#7B1FA2"),
            ),
        )
    }

    if (style.getSource(SURVEY_LINE_SOURCE) == null) {
        style.addSource(GeoJsonSource(SURVEY_LINE_SOURCE))
        style.addLayer(
            LineLayer(SURVEY_LINE_LAYER, SURVEY_LINE_SOURCE).withProperties(
                PropertyFactory.lineColor("#7B1FA2"),
                PropertyFactory.lineWidth(3f),
            ),
        )
    }

    if (style.getSource(SURVEY_TRANSECT_SOURCE) == null) {
        style.addSource(GeoJsonSource(SURVEY_TRANSECT_SOURCE))
        style.addLayer(
            LineLayer(SURVEY_TRANSECT_LAYER, SURVEY_TRANSECT_SOURCE).withProperties(
                PropertyFactory.lineColor("#E040FB"),
                PropertyFactory.lineWidth(2.5f),
            ),
        )
    }
}

internal fun shadedArea(survey: Survey): List<TrackPoint> = if (survey.shape == SHAPE_AREA) survey.area else survey.outline

fun surveyAreaFeatures(surveys: List<Survey>): FeatureCollection {
    val features = surveys.mapNotNull { survey ->
        val area = shadedArea(survey)
        if (area.size < 3) return@mapNotNull null
        val ring = area.map { Point.fromLngLat(it.longitude, it.latitude) }
        val closed = if (ring.first() == ring.last()) ring else ring + ring.first()
        Feature.fromGeometry(Polygon.fromLngLats(listOf(closed))).apply { addBooleanProperty(SURVEY_COLLISION, survey.collides) }
    }
    return FeatureCollection.fromFeatures(features)
}

fun surveyLineFeatures(surveys: List<Survey>): FeatureCollection {
    val features = surveys.filter { it.shape == SHAPE_LINE }.mapNotNull { survey ->
        if (survey.area.size < 2) return@mapNotNull null
        Feature.fromGeometry(
            LineString.fromLngLats(survey.area.map { Point.fromLngLat(it.longitude, it.latitude) }),
        )
    }
    return FeatureCollection.fromFeatures(features)
}

fun flownRoute(survey: Survey): List<TrackPoint> = when {
    survey.transects.size >= 2 -> survey.transects
    survey.flightLoop.size >= 2 -> closedLoop(survey.flightLoop)
    else -> emptyList()
}

private fun closedLoop(points: List<TrackPoint>): List<TrackPoint> =
    if (points.first() == points.last()) points else points + points.first()

fun surveyTransectFeatures(surveys: List<Survey>): FeatureCollection {
    val features = surveys.mapNotNull { survey ->
        val route = flownRoute(survey)
        if (route.size < 2) return@mapNotNull null
        val line = route.map { Point.fromLngLat(it.longitude, it.latitude) }
        Feature.fromGeometry(LineString.fromLngLats(line))
    }
    return FeatureCollection.fromFeatures(features)
}

fun renderSurveys(style: Style, surveys: List<Survey>) {
    (style.getSource(SURVEY_AREA_SOURCE) as? GeoJsonSource)?.setGeoJson(surveyAreaFeatures(surveys))
    (style.getSource(SURVEY_LINE_SOURCE) as? GeoJsonSource)?.setGeoJson(surveyLineFeatures(surveys))
    (style.getSource(SURVEY_TRANSECT_SOURCE) as? GeoJsonSource)
        ?.setGeoJson(surveyTransectFeatures(surveys))
}
