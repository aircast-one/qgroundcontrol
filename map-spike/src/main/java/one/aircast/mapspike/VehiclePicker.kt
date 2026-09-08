package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

private const val VEHICLES_ROOT = "vehicles"
private const val VEHICLE_LIST = "$VEHICLES_ROOT.vehicles"

data class VehicleEntry(val index: Int, val id: Int, val active: Boolean)

fun vehicleEntries(json: JSONObject?, activeId: Int): List<VehicleEntry> {
    val elements = json?.optJSONArray("elements") ?: return emptyList()

    return (0 until elements.length()).mapNotNull { index ->
        val element = elements.optJSONObject(index) ?: return@mapNotNull null
        val id = element.optInt("id", -1).takeIf { it >= 0 } ?: return@mapNotNull null
        VehicleEntry(index = index, id = id, active = id == activeId)
    }
}

fun vehicleSummary(entries: List<VehicleEntry>): String? {
    if (entries.size < 2) {
        return null
    }
    val active = entries.firstOrNull { it.active } ?: return null
    return entries.sortedBy { it.id }.joinToString(", ") { "Vehicle ${it.id}" } + " \u00b7 ${active.id} active"
}

object VehicleBridge {
    fun entries(activeId: Int): List<VehicleEntry> =
        vehicleEntries(
            runCatching { JSONObject(QGCBridge.get(VEHICLE_LIST)) }.getOrNull(),
            activeId,
        )
}
