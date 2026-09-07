package one.aircast.mapspike

import org.maplibre.android.maps.Style
import org.maplibre.android.style.layers.CircleLayer
import org.maplibre.android.style.layers.LineLayer
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.layers.SymbolLayer
import org.maplibre.android.style.sources.GeoJsonSource
import org.maplibre.geojson.Feature
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.LineString
import org.maplibre.geojson.Point

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
