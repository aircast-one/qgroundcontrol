package one.aircast.mapspike

import org.maplibre.android.maps.Style
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

const val MISSION_SOURCE = "aircast-mission"
const val MISSION_LAYER = "aircast-mission-layer"
const val MISSION_DOT_LAYER = "aircast-mission-dot-layer"
const val MISSION_PATH_SOURCE = "aircast-mission-path"
const val MISSION_PATH_LAYER = "aircast-mission-path-layer"

const val WAYPOINT_ID_PROPERTY = "waypointId"
private const val WAYPOINT_LABEL_PROPERTY = "label"

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
                PropertyFactory.circleColor("#FFB300"),
                PropertyFactory.circleRadius(13f),
                PropertyFactory.circleStrokeColor("#37474F"),
                PropertyFactory.circleStrokeWidth(2f),
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
                PropertyFactory.textAllowOverlap(true),
                PropertyFactory.textIgnorePlacement(true),
            ),
        )
    }
}

fun missionFeatures(items: List<MissionItem>): FeatureCollection {
    val features = items.map { item ->
        Feature.fromGeometry(Point.fromLngLat(item.longitude, item.latitude)).apply {
            addNumberProperty(WAYPOINT_ID_PROPERTY, item.index)
            addStringProperty(WAYPOINT_LABEL_PROPERTY, item.sequence.toString())
        }
    }
    return FeatureCollection.fromFeatures(features)
}

fun missionPath(items: List<MissionItem>): Feature? {
    val points = items.map { Point.fromLngLat(it.longitude, it.latitude) }
    if (points.size < 2) {
        return null
    }
    return Feature.fromGeometry(LineString.fromLngLats(points))
}

fun renderMission(style: Style, items: List<MissionItem>) {
    (style.getSource(MISSION_SOURCE) as? GeoJsonSource)?.setGeoJson(missionFeatures(items))

    val path = missionPath(items)
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
const val RALLY_SOURCE = "aircast-rally"
const val RALLY_LAYER = "aircast-rally-layer"

fun installFenceLayers(style: Style) {
    if (style.getSource(FENCE_SOURCE) == null) {
        style.addSource(GeoJsonSource(FENCE_SOURCE))
        style.addLayer(
            FillLayer(FENCE_FILL_LAYER, FENCE_SOURCE).withProperties(
                PropertyFactory.fillColor("#42A5F5"),
                PropertyFactory.fillOpacity(0.15f),
            ),
        )
        style.addLayer(
            LineLayer(FENCE_LINE_LAYER, FENCE_SOURCE).withProperties(
                PropertyFactory.lineColor("#42A5F5"),
                PropertyFactory.lineWidth(2.5f),
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

const val CIRCLE_INDEX_PROPERTY = "circleIndex"

private fun ringFeature(vertices: List<TrackPoint>): Feature {
    val ring = vertices.map { Point.fromLngLat(it.longitude, it.latitude) }
    val closed = if (ring.first() == ring.last()) ring else ring + ring.first()
    return Feature.fromGeometry(Polygon.fromLngLats(listOf(closed)))
}

// A circle carries its index so tapping its ring can find it again. A polygon
// needs no tag: it is selected by one of its vertex handles.
fun fenceFeatures(
    polygons: List<FencePolygon>,
    circles: List<FencePolygon> = emptyList(),
): FeatureCollection {
    val polygonFeatures = polygons.map { ringFeature(it.vertices) }
    val circleFeatures = circles.map { circle ->
        ringFeature(circle.vertices).apply { addNumberProperty(CIRCLE_INDEX_PROPERTY, circle.index) }
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

fun renderFences(
    style: Style,
    polygons: List<FencePolygon>,
    rally: List<RallyPoint>,
    circles: List<FencePolygon> = emptyList(),
) {
    (style.getSource(FENCE_SOURCE) as? GeoJsonSource)?.setGeoJson(fenceFeatures(polygons, circles))
    (style.getSource(RALLY_SOURCE) as? GeoJsonSource)?.setGeoJson(rallyFeatures(rally))
}

const val FENCE_HANDLE_SOURCE = "aircast-fence-handles"
const val FENCE_HANDLE_LAYER = "aircast-fence-handle-layer"

const val POLYGON_INDEX_PROPERTY = "polygonIndex"
const val VERTEX_INDEX_PROPERTY = "vertexIndex"
const val HANDLE_KIND_PROPERTY = "handleKind"

const val HANDLE_KIND_FENCE = "fence"
const val HANDLE_KIND_SURVEY = "survey"

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

// Fence and survey handles share one source. They behave identically under the
// finger, and only the path they write differs.
fun vertexHandleFeatures(
    polygons: List<FencePolygon>,
    surveys: List<Survey>,
): FeatureCollection {
    val fence = polygons.flatMap { handleFeatures(HANDLE_KIND_FENCE, it.index, it.vertices) }
    val survey = surveys.flatMap { handleFeatures(HANDLE_KIND_SURVEY, it.index, it.area) }
    return FeatureCollection.fromFeatures(fence + survey)
}

fun renderVertexHandles(style: Style, polygons: List<FencePolygon>, surveys: List<Survey>) {
    (style.getSource(FENCE_HANDLE_SOURCE) as? GeoJsonSource)
        ?.setGeoJson(vertexHandleFeatures(polygons, surveys))
}

const val SURVEY_AREA_SOURCE = "aircast-survey-area"
const val SURVEY_AREA_LAYER = "aircast-survey-area-layer"
const val SURVEY_TRANSECT_SOURCE = "aircast-survey-transects"
const val SURVEY_TRANSECT_LAYER = "aircast-survey-transect-layer"

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
    val features = surveys.mapNotNull { survey ->
        if (survey.area.size < 3) return@mapNotNull null
        val ring = survey.area.map { Point.fromLngLat(it.longitude, it.latitude) }
        val closed = if (ring.first() == ring.last()) ring else ring + ring.first()
        Feature.fromGeometry(Polygon.fromLngLats(listOf(closed)))
    }
    return FeatureCollection.fromFeatures(features)
}

fun surveyTransectFeatures(surveys: List<Survey>): FeatureCollection {
    val features = surveys.mapNotNull { survey ->
        if (survey.transects.size < 2) return@mapNotNull null
        val line = survey.transects.map { Point.fromLngLat(it.longitude, it.latitude) }
        Feature.fromGeometry(LineString.fromLngLats(line))
    }
    return FeatureCollection.fromFeatures(features)
}

fun renderSurveys(style: Style, surveys: List<Survey>) {
    (style.getSource(SURVEY_AREA_SOURCE) as? GeoJsonSource)?.setGeoJson(surveyAreaFeatures(surveys))
    (style.getSource(SURVEY_TRANSECT_SOURCE) as? GeoJsonSource)
        ?.setGeoJson(surveyTransectFeatures(surveys))
}
