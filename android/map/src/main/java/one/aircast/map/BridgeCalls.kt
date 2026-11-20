package one.aircast.map

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge
import org.json.JSONArray

fun coordinateJson(latitude: Double, longitude: Double, altitudeMetres: Double = 0.0): String =
    "{\"latitude\":$latitude,\"longitude\":$longitude,\"altitude\":$altitudeMetres}"

fun coordinateJson(at: TrackPoint): String = coordinateJson(at.latitude, at.longitude)

fun settingJson(value: String): String = "{\"value\":$value}"

fun setOk(path: String, value: String): Boolean =
    runCatching { JSONObject(QGCBridge.set(path, value)).optBoolean("ok") }.getOrDefault(false)

const val PLAN_UNDO = "plan.undo"

fun invokeOk(path: String, args: String = "[]"): Boolean =
    runCatching { JSONObject(QGCBridge.invoke(path, args)).optBoolean("ok") }.getOrDefault(false)

fun refusalIn(answer: JSONObject?, fallback: String): String? =
    if (answer?.optBoolean("ok") == true) null else answer?.optText("reason")?.ifBlank { null } ?: fallback

fun invokeRefusal(path: String, args: String, fallback: String): String? =
    refusalIn(runCatching { JSONObject(QGCBridge.invoke(path, args)) }.getOrNull(), fallback)

fun JSONObject.optText(key: String): String = if (isNull(key)) "" else optString(key)

fun JSONArray.optText(index: Int): String = if (isNull(index)) "" else optString(index)
