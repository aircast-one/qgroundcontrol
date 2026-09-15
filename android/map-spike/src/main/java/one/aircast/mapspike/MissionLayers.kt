package one.aircast.mapspike

import org.maplibre.android.maps.Style
import org.maplibre.android.style.expressions.Expression
import org.maplibre.android.style.layers.CircleLayer
import org.maplibre.android.style.layers.FillLayer
import org.maplibre.android.style.layers.LineLayer
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.layers.SymbolLayer
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.LineString
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

const val WAYPOINT_ID_PROPERTY = "waypointId"
internal const val WAYPOINT_LABEL_PROPERTY = "label"
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
    }
}

fun crowded(itemCount: Int): Boolean = itemCount > CROWDED_ITEMS

fun waypointLabel(sequence: Int, crowded: Boolean): String =
    if (crowded) "" else sequence.toString()

fun markerRadius(crowded: Boolean, selected: Boolean): Double =
    if (crowded && !selected) CROWDED_RADIUS else MARKER_RADIUS

fun markerStroke(crowded: Boolean, selected: Boolean): Double = when {
    selected -> SELECTED_STROKE
    crowded -> CROWDED_STROKE
    else -> MARKER_STROKE
}

fun missionFeatures(items: List<MissionItem>, selectedIndex: Int? = null): FeatureCollection {
    val crowded = crowded(items.size)
    val features = items.map { item ->
        Feature.fromGeometry(Point.fromLngLat(item.longitude, item.latitude)).apply {
            addNumberProperty(WAYPOINT_ID_PROPERTY, item.index)
            addStringProperty(WAYPOINT_LABEL_PROPERTY, waypointLabel(item.sequence, crowded))
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

fun missionPath(items: List<MissionItem>, linkStartToHome: Boolean): Feature? {
    val flown = (if (linkStartToHome) items else items.filterNot { it.index == 0 })
        .filter { it.routed }
    val points = flown.flatMap { item ->
        listOfNotNull(
            Point.fromLngLat(item.longitude, item.latitude),
            item.exit?.let { Point.fromLngLat(it.longitude, it.latitude) },
        )
    }
    if (points.size < 2) {
        return null
    }
    return Feature.fromGeometry(LineString.fromLngLats(points))
}

fun renderMission(
    style: Style,
    items: List<MissionItem>,
    linkStartToHome: Boolean,
    selectedIndex: Int? = null,
) {
    (style.getSource(MISSION_SOURCE) as? GeoJsonSource)
        ?.setGeoJson(missionFeatures(items, selectedIndex))

    val path = missionPath(items, linkStartToHome)
    val pathSource = style.getSource(MISSION_PATH_SOURCE) as? GeoJsonSource ?: return
    if (path == null) {
        pathSource.setGeoJson(FeatureCollection.fromFeatures(emptyList()))
    } else {
        pathSource.setGeoJson(path)
    }
}

const val FENCE_SOURCE = "aircast-fence"
const val FENCE_FILL_LAYER = "aircast-fence-fill"
const val FENCE_LINE_LAYER = "aircast-fence-line"
const val FIRMWARE_FENCE_SOURCE = "aircast-firmware-fence"
const val FIRMWARE_FENCE_LAYER = "aircast-firmware-fence-line"
const val RALLY_SOURCE = "aircast-rally"
const val RALLY_LAYER = "aircast-rally-layer"

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
    }
}

const val GCS_SOURCE = "aircast-gcs"
const val GCS_LAYER = "aircast-gcs-layer"

fun operatorFeatures(point: TrackPoint?): FeatureCollection =
    FeatureCollection.fromFeatures(
        listOfNotNull(point).map { Feature.fromGeometry(Point.fromLngLat(it.longitude, it.latitude)) },
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
) {
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
        ),
    )
}

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
    val places = landings.flatMap { landingHandleFeatures(it) }
    return FeatureCollection.fromFeatures(fence + survey + centres + places)
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
) {
    (style.getSource(FENCE_HANDLE_SOURCE) as? GeoJsonSource)
        ?.setGeoJson(vertexHandleFeatures(polygons, surveys, circles, landings))
}

const val SURVEY_AREA_SOURCE = "aircast-survey-area"
const val SURVEY_AREA_LAYER = "aircast-survey-area-layer"
const val SURVEY_TRANSECT_SOURCE = "aircast-survey-transects"
const val SURVEY_TRANSECT_LAYER = "aircast-survey-transect-layer"
const val LANDING_PATH_SOURCE = "aircast-landing-path"
const val LANDING_PATH_LAYER = "aircast-landing-path-layer"
const val LANDING_LOITER_SOURCE = "aircast-landing-loiter"
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

fun renderMidpoints(style: Style, polygons: List<FencePolygon>, surveys: List<Survey>) {
    (style.getSource(MIDPOINT_SOURCE) as? GeoJsonSource)
        ?.setGeoJson(midpointFeatures(polygons.map { it.editable } + surveys.map { it.editable }))
}

fun installLandingLayers(style: Style) {
    if (style.getSource(LANDING_LOITER_SOURCE) == null) {
        style.addSource(GeoJsonSource(LANDING_LOITER_SOURCE))
        style.addLayer(
            LineLayer(LANDING_LOITER_LAYER, LANDING_LOITER_SOURCE).withProperties(
                PropertyFactory.lineColor("#26C6DA"),
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
                PropertyFactory.fillColor("#AB47BC"),
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

fun surveyAreaFeatures(surveys: List<Survey>): FeatureCollection {
    val features = surveys.filter { it.shape == SHAPE_AREA }.mapNotNull { survey ->
        if (survey.area.size < 3) return@mapNotNull null
        val ring = survey.area.map { Point.fromLngLat(it.longitude, it.latitude) }
        val closed = if (ring.first() == ring.last()) ring else ring + ring.first()
        Feature.fromGeometry(Polygon.fromLngLats(listOf(closed)))
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
