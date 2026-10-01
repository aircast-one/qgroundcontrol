package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
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
    }
}
