package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceViewFixturesTest {
    private fun served(page: String): JSONObject = JSONObject(javaClass.getResource("/aircastd/$page.json")!!.readText())

    @Test
    fun `every device page reads what aircastd serves`() {
        assertEquals("Ready to fly", parseDeviceStatus(served("aircraft"))?.headline)
        assertTrue(parseTelemetry(served("telemetry"))!!.endpoints.isNotEmpty())
        assertTrue(parseConnect(served("connect"))!!.choices.isNotEmpty())
        assertEquals(emptyList<Any>(), parseFlights(served("flights"))?.flights)
        assertTrue(parseDeviceSettings(served("device"))!!.stats.isNotEmpty())
        assertTrue(parseEngineering(served("engineering")) != null)
        assertTrue(parseVideo(served("video")) != null)
        assertEquals("cam2", parseDetection(served("detection"))?.cameras?.first()?.name)
        assertTrue(parseRecordings(served("recordings"))!!.limit.options.isNotEmpty())
        assertEquals("Wi-Fi (wlan0)", parseNetwork(served("network"))?.uplink?.title)
        assertEquals("cam2", parseCameras(served("cameras"))?.cameras?.first()?.name)
        assertEquals("network", parseDeviceSetup(served("setup"))?.steps?.first()?.key)
    }
}
