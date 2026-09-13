package one.aircast.mapspike

import org.mavlink.qgroundcontrol.QGCBridge
import org.json.JSONObject

const val GCS_POSITION_VIEW = "view.gcsPosition"

object OperatorBridge {
    fun read(): JSONObject? =
        runCatching { JSONObject(QGCBridge.get(GCS_POSITION_VIEW)) }.getOrNull()
}

fun operatorPoint(view: JSONObject?): TrackPoint? =
    view?.takeIf { it.optBoolean("usable") }?.let(::coordinate)
