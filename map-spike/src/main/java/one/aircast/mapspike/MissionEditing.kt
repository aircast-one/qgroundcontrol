package one.aircast.mapspike

import android.annotation.SuppressLint
import android.graphics.PointF
import android.graphics.RectF
import android.view.MotionEvent
import org.maplibre.android.maps.MapLibreMap
import org.maplibre.android.maps.MapView
import org.maplibre.android.maps.Style

private const val HIT_RADIUS_PX = 44f
private const val DRAG_WRITE_INTERVAL_MS = 120L
private const val TAP_SLOP_PX = 20f

sealed interface MapHit {
    data class Waypoint(val index: Int) : MapHit

    data class FenceVertex(val polygon: Int, val vertex: Int) : MapHit

    data class SurveyVertex(val item: Int, val vertex: Int) : MapHit

    data class Rally(val index: Int) : MapHit

    data class Circle(val index: Int) : MapHit

    data class CircleCentre(val index: Int) : MapHit
}

// Fence handles win a tie. They sit on the fence outline, which a waypoint can
// easily overlap, and a handle is the smaller target of the two.
// queryRenderedFeatures returns everything the box touches in source order, so
// firstOrNull picks by list position rather than by distance. Where markers sit
// close together — a launch point and the first waypoint routinely do — that
// grabs whichever was drawn first, and the wrong item moves. Fills are exempt:
// being inside one is not a matter of degree.
// Split out from the projection so the rule can be tested without a map. A null
// entry is something that did not project to a point; it sorts last but still
// wins if nothing else is there, which is what the map-backed version did before
// and what the callers rely on to read properties off a non-point feature.
internal fun nearestIndex(x: Float, y: Float, points: List<Pair<Float, Float>?>): Int? =
    points.indices.minByOrNull { index ->
        points[index]?.let { (px, py) ->
            val dx = px - x
            val dy = py - y
            dx * dx + dy * dy
        } ?: Float.MAX_VALUE
    }

private fun nearest(
    map: MapLibreMap,
    features: List<org.maplibre.geojson.Feature>,
    x: Float,
    y: Float,
): org.maplibre.geojson.Feature? = nearestIndex(
    x,
    y,
    features.map { feature ->
        (feature.geometry() as? org.maplibre.geojson.Point)?.let { point ->
            map.projection.toScreenLocation(
                org.maplibre.android.geometry.LatLng(point.latitude(), point.longitude()),
            ).let { it.x to it.y }
        }
    },
)?.let { features[it] }

fun hitTest(map: MapLibreMap, x: Float, y: Float): MapHit? {
    val box = RectF(x - HIT_RADIUS_PX, y - HIT_RADIUS_PX, x + HIT_RADIUS_PX, y + HIT_RADIUS_PX)

    nearest(map, map.queryRenderedFeatures(box, FENCE_HANDLE_LAYER), x, y)?.let { feature ->
        val owner = feature.getNumberProperty(POLYGON_INDEX_PROPERTY)?.toInt()
        val vertex = feature.getNumberProperty(VERTEX_INDEX_PROPERTY)?.toInt()
        val kind = feature.getStringProperty(HANDLE_KIND_PROPERTY)
        if (owner != null && vertex != null) {
            return when (kind) {
                HANDLE_KIND_SURVEY -> MapHit.SurveyVertex(owner, vertex)
                HANDLE_KIND_CIRCLE -> MapHit.CircleCentre(owner)
                else -> MapHit.FenceVertex(owner, vertex)
            }
        }
    }

    nearest(map, map.queryRenderedFeatures(box, RALLY_LAYER), x, y)?.let { feature ->
        feature.getNumberProperty(RALLY_INDEX_PROPERTY)?.toInt()?.let { return MapHit.Rally(it) }
    }

    map.queryRenderedFeatures(box, FENCE_FILL_LAYER).firstOrNull()?.let { feature ->
        feature.getNumberProperty(CIRCLE_INDEX_PROPERTY)?.toInt()?.let { return MapHit.Circle(it) }
    }

    return nearest(map, map.queryRenderedFeatures(box, MISSION_DOT_LAYER, MISSION_LAYER), x, y)
        ?.getNumberProperty(WAYPOINT_ID_PROPERTY)
        ?.toInt()
        ?.let { MapHit.Waypoint(it) }
}

@SuppressLint("ClickableViewAccessibility")
fun attachMissionEditing(
    mapView: MapView,
    map: MapLibreMap,
    style: Style,
    onAdd: (Double, Double) -> Unit,
    onMove: (MapHit, Double, Double) -> Unit,
    onSelected: (MapHit?) -> Unit = {},
) {
    var dragging: MapHit? = null
    var downX = 0f
    var downY = 0f
    var moved = false
    var lastWriteAt = 0L

    // Long press adds. It never deletes, because a slow drag begins with a long
    // press and deleting the waypoint the user meant to move is unrecoverable.
    map.addOnMapLongClickListener { latLng ->
        onAdd(latLng.latitude, latLng.longitude)
        true
    }

    mapView.setOnTouchListener { view, event ->
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                val hit = hitTest(map, event.x, event.y)
                if (hit == null) {
                    // A drag the system steals never delivers its release, which
                    // would otherwise leave the map's own gestures switched off.
                    map.uiSettings.setAllGesturesEnabled(true)
                    false
                } else {
                    dragging = hit
                    downX = event.x
                    downY = event.y
                    moved = false
                    map.uiSettings.setAllGesturesEnabled(false)
                    true
                }
            }

            MotionEvent.ACTION_MOVE -> {
                val hit = dragging ?: return@setOnTouchListener false
                if (!moved && (kotlin.math.abs(event.x - downX) > TAP_SLOP_PX ||
                        kotlin.math.abs(event.y - downY) > TAP_SLOP_PX)
                ) {
                    moved = true
                }
                // Every write is a blocking trip into the Qt thread, and a drag
                // delivers touch moves far faster than those can complete. Writing
                // one per event floods the bridge with overlapping calls whose
                // order is not guaranteed, so the item can settle somewhere the
                // finger never was. Intermediate positions are dropped instead;
                // the release below always writes the real one.
                val now = event.eventTime
                if (moved && now - lastWriteAt >= DRAG_WRITE_INTERVAL_MS) {
                    lastWriteAt = now
                    val target = map.projection.fromScreenLocation(PointF(event.x, event.y))
                    onMove(hit, target.latitude, target.longitude)
                }
                true
            }

            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                val hit = dragging
                dragging = null
                map.uiSettings.setAllGesturesEnabled(true)
                if (hit == null) {
                    false
                } else {
                    if (moved) {
                        val target = map.projection.fromScreenLocation(PointF(event.x, event.y))
                        onMove(hit, target.latitude, target.longitude)
                    } else {
                        onSelected(hit)
                        view.performClick()
                    }
                    true
                }
            }

            else -> false
        }
    }
}
