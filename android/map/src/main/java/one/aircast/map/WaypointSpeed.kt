package one.aircast.map

import org.json.JSONObject

data class WaypointSpeed(val specified: Boolean, val value: Double?, val units: String, val path: String, val specifyPath: String)

fun waypointSpeed(view: JSONObject?): WaypointSpeed? =
    view?.optJSONObject("speedSection")?.takeIf { it.optBoolean("available") }?.let {
        WaypointSpeed(
            specified = it.optBoolean("specified"),
            value = if (it.isNull("value")) null else it.optDouble("value"),
            units = it.optText("units"),
            path = it.optText("path"),
            specifyPath = it.optText("specifyPath"),
        )
    }

data class WaypointHold(val seconds: Double, val units: String, val path: String)

fun waypointHold(view: JSONObject?): WaypointHold? =
    view?.optJSONObject("hold")?.let { WaypointHold(it.optDouble("value", 0.0), it.optText("units"), it.optText("path")) }?.takeIf { it.path.isNotBlank() }
