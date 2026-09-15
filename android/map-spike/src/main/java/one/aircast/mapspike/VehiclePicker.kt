package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

object VehicleBridge {
    var lastRefusal: String? = null
        private set

    fun askFor(id: Int): Boolean =
        runCatching {
            val answer = JSONObject(QGCBridge.invoke("vehicles.setActive", "[$id]"))
            lastRefusal = answer.optText("reason").takeIf { it.isNotBlank() }
            answer.optBoolean("ok")
        }.onFailure { lastRefusal = "bridge threw: ${it.message}" }.getOrDefault(false)
}
