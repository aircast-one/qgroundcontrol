package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceWifiWizardTest {
    @Test
    fun `a Wi-Fi join is checked before the hotspot is given up`() {
        assertEquals(DeviceRefusal("Enter your network name first.", "ssid"), wifiJoinProblem("  ", "", ""))
        assertTrue(wifiJoinProblem("workshop", "", "workshop")?.message.orEmpty().contains("secured network"))
        assertNull(wifiJoinProblem("typed-by-hand", "", ""))
        assertTrue(wifiJoinProblem("workshop", "short", "")?.message.orEmpty().contains("8–63 characters"))
        assertEquals("psk", wifiJoinProblem("workshop", "x".repeat(64), "")?.field)
        assertNull(wifiJoinProblem("workshop", "supersecret", "workshop"))
    }

    @Test
    fun `a join that lost the link says how to find the device, a refused one hides those steps`() {
        val join = WifiJoinAttempt(1, "workshop", "Switching…", true)
        assertTrue(joinOutcome(join, null).status.contains("Connected to “workshop”"))
        val lost = joinOutcome(join, NO_ANSWER)
        assertTrue(lost.instructions)
        assertTrue(lost.status.contains("Lost contact"))
        assertEquals(join.copy(instructions = false, status = "✗ SSID is too long"), joinOutcome(join, DeviceRefusal("SSID is too long", "ssid")))
    }

    @Test
    fun `reads the networks a scan found`() {
        assertEquals(listOf(WifiNetwork("workshop", 82, secured = true, active = false)), parseWifiScan(JSONObject("""{"networks":[{"ssid":"workshop","signal":82,"secured":true}]}""")))
        assertNull(parseWifiScan(null))
    }
}
