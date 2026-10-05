package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class AutoLinksTest {
    @Test
    fun `connected automatic links are listed, saved and idle ones are not`() {
        val view = JSONObject("""{"links":[
            {"name":"UDP Link (AutoConnect)","displaySummary":"UDP port 14550","dynamic":true,"connected":true,"heardVehicle":false},
            {"name":"TCP 127.0.0.1:5771","displaySummary":"127.0.0.1:5771","dynamic":true,"connected":true,"heardVehicle":true},
            {"name":"SiK","displaySummary":"ttyUSB0","dynamic":false,"connected":true,"heardVehicle":true},
            {"name":"Old","displaySummary":"x","dynamic":true,"connected":false,"heardVehicle":false}]}""")

        val links = autoLinks(view)
        assertEquals(listOf("UDP Link (AutoConnect)", "TCP 127.0.0.1:5771"), links.map { it.name })
        assertEquals(listOf("Listening", "Vehicle"), links.map(::autoLinkStatus))
    }

    @Test
    fun `a link with no address reads without a stray separator, and a mock link reads as simulated`() {
        assertEquals("automatic", autoLinkSubtitle(AutoLink("Serial", "", false)))
        assertEquals("UDP port 14550 \u00b7 automatic", autoLinkSubtitle(AutoLink("UDP", "UDP port 14550", false)))
        assertEquals("Simulated", autoLinkSubtitle(AutoLink("PX4 MultiRotor MockLink", "", true, MOCK_LINK)))
    }
}
