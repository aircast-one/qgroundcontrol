package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class BatteryReturnTest {
    private fun battery(state: Int, power: String, current: String) = JSONObject(
        """{"available":true,"packs":[{"chargeState":$state,"facts":[{"name":"instantPower","value":$power},{"name":"current","value":$current}]}]}""",
    )

    @Test
    fun `a critical or worse pack offers Return`() {
        assertTrue(batteryReturnOffered(battery(3, "120", "10")))
        assertTrue(batteryReturnOffered(battery(6, "120", "10")))
        assertFalse(batteryReturnOffered(battery(2, "120", "10")))
        assertFalse(batteryReturnOffered(null))
    }

    @Test
    fun `total draw sums watts, else amps`() {
        assertEquals("120W", totalDraw(battery(1, "119.6", "10")))
        assertEquals("10.0A", totalDraw(battery(1, "null", "10")))
        assertNull(totalDraw(battery(1, "null", "null")))
    }
}
