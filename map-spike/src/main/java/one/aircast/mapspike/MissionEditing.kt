package one.aircast.mapspike

import android.annotation.SuppressLint
import android.graphics.PointF
import android.graphics.RectF
import android.view.MotionEvent
import org.maplibre.android.maps.MapLibreMap
import org.maplibre.android.maps.MapView
import org.maplibre.android.maps.Style

private const val HIT_RADIUS_PX = 44f
private const val TAP_SLOP_PX = 20f

fun hitWaypointId(map: MapLibreMap, x: Float, y: Float): Int? {
    val box = RectF(x - HIT_RADIUS_PX, y - HIT_RADIUS_PX, x + HIT_RADIUS_PX, y + HIT_RADIUS_PX)
    val features = map.queryRenderedFeatures(box, MISSION_DOT_LAYER, MISSION_LAYER)
    return features.firstOrNull()?.getNumberProperty(WAYPOINT_ID_PROPERTY)?.toInt()
}

@SuppressLint("ClickableViewAccessibility")
fun attachMissionEditing(
    mapView: MapView,
    map: MapLibreMap,
    style: Style,
    mission: MissionModel,
    onChanged: () -> Unit,
    onSelected: (Int?) -> Unit = {},
) {
    var draggingId: Int? = null
    var downX = 0f
    var downY = 0f
    var moved = false

    // Long press adds. It never deletes, because a slow drag begins with a long
    // press and deleting the waypoint the user meant to move is unrecoverable.
    map.addOnMapLongClickListener { latLng ->
        mission.add(latLng.latitude, latLng.longitude)
        onChanged()
        true
    }

    mapView.setOnTouchListener { view, event ->
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                val id = hitWaypointId(map, event.x, event.y)
                if (id == null) {
                    false
                } else {
                    draggingId = id
                    downX = event.x
                    downY = event.y
                    moved = false
                    map.uiSettings.setAllGesturesEnabled(false)
                    true
                }
            }

            MotionEvent.ACTION_MOVE -> {
                val id = draggingId ?: return@setOnTouchListener false
                if (!moved && (kotlin.math.abs(event.x - downX) > TAP_SLOP_PX ||
                        kotlin.math.abs(event.y - downY) > TAP_SLOP_PX)
                ) {
                    moved = true
                }
                if (moved) {
                    val target = map.projection.fromScreenLocation(PointF(event.x, event.y))
                    if (mission.move(id, target.latitude, target.longitude)) {
                        onChanged()
                    }
                }
                true
            }

            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                val id = draggingId
                draggingId = null
                map.uiSettings.setAllGesturesEnabled(true)
                if (id == null) {
                    false
                } else {
                    if (!moved) {
                        onSelected(id)
                        view.performClick()
                    }
                    true
                }
            }

            else -> false
        }
    }
}
