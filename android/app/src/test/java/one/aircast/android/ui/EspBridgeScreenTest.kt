package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class EspBridgeScreenTest {
    @Test
    fun the_bridge_reads_settings_and_grouped_counters() {
        assertNull(espBridge(JSONObject("""{"available":false}""")))
        val bridge = espBridge(JSONObject("""{"available":true,"modeIndex":null,"channel":6,"ssid":"PixRacer","password":"pixracer","ssidSta":null,"passwordSta":null,
            "baudRates":[57600,921600],"baudIndex":1,"hostPort":{"valueString":"14550"},
            "status":{"vehicle":{"received":1234567,"lost":0,"sent":null},"bridge":{},"qgc":{}}}"""))!!
        assertNull(bridge.modeIndex)
        assertEquals(LinkCounts("1,234,567", "0", ""), bridge.vehicle)
        assertEquals("14550", bridge.hostPort)
        assertFalse("ESP8266Component.qml enables the STA fields only in station mode", stationFieldsEnabled(bridge))
        assertTrue(stationFieldsEnabled(bridge.copy(modeIndex = 1)))
    }

    @Test
    fun restart_waits_for_the_bridge_like_controller_busy() {
        assertFalse(espBridge(JSONObject("""{"available":true}"""))!!.busy)
        assertTrue(espBridge(JSONObject("""{"available":true,"busy":true}"""))!!.busy)
    }

    @Test
    fun the_qgc_udp_port_is_typed_within_the_validator_range() {
        val bridge = espBridge(JSONObject("""{"available":true,"hostPort":{"valueString":"14550","path":"vehicle.parameterManager.getParameter(240,WIFI_UDP_HPORT)"}}"""))!!
        assertEquals("vehicle.parameterManager.getParameter(240,WIFI_UDP_HPORT)", bridge.hostPortPath)
        assertEquals(listOf(null, 1024, 65535, null), listOf("1023", "1024", "65535", "65536").map(::hostPortTyped))
    }
}
