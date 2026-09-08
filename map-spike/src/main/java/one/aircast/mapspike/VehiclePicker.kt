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

object VehicleBridge {
    fun entries(activeId: Int): List<VehicleEntry> =
        vehicleEntries(
            runCatching { JSONObject(QGCBridge.get(VEHICLE_LIST)) }.getOrNull(),
            activeId,
        )

    // activeVehicle takes a Vehicle*, so the write is an @path naming the entry
    // in the manager's own list rather than a copy of it. The bridge resolves the
    // reference and type-checks it against the property.
    fun makeActive(index: Int): Boolean =
        runCatching {
            JSONObject(
                QGCBridge.set(
                    "$VEHICLES_ROOT.activeVehicle",
                    "{\"value\":\"@$VEHICLE_LIST.$index\"}",
                ),
            ).optBoolean("ok")
        }.getOrDefault(false)
}
