package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

fun coordinateJson(latitude: Double, longitude: Double): String =
    "{\"latitude\":$latitude,\"longitude\":$longitude,\"altitude\":0}"

fun coordinateJson(at: TrackPoint): String = coordinateJson(at.latitude, at.longitude)

fun settingJson(value: String): String = "{\"value\":$value}"

fun setOk(path: String, value: String): Boolean =
    runCatching { JSONObject(QGCBridge.set(path, value)).optBoolean("ok") }.getOrDefault(false)

fun invokeOk(path: String, args: String = "[]"): Boolean =
    runCatching { JSONObject(QGCBridge.invoke(path, args)).optBoolean("ok") }.getOrDefault(false)
