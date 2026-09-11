package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

private const val VEHICLES_VIEW = "view.vehicles"

data class VehicleEntry(val id: Int, val active: Boolean)

fun vehicleEntries(json: JSONObject?): List<VehicleEntry> {
    val listed = json?.optJSONArray("vehicles") ?: return emptyList()

    return (0 until listed.length()).mapNotNull { index ->
        val element = listed.optJSONObject(index) ?: return@mapNotNull null
        val id = element.optInt("id", -1).takeIf { it >= 0 } ?: return@mapNotNull null
        VehicleEntry(id = id, active = element.optBoolean("active"))
    }
}

object VehicleBridge {
    var lastRefusal: String? = null
        private set

    fun askFor(id: Int): Boolean =
        runCatching {
            val answer = JSONObject(QGCBridge.invoke("vehicles.setActive", "[$id]"))
            lastRefusal = answer.optString("reason").takeIf { it.isNotBlank() }
            answer.optBoolean("ok")
        }.onFailure { lastRefusal = "bridge threw: ${it.message}" }.getOrDefault(false)

    fun entries(): List<VehicleEntry> =
        vehicleEntries(runCatching { JSONObject(QGCBridge.get(VEHICLES_VIEW)) }.getOrNull())
}
