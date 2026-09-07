package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RcSignalTest {
    @Test
    fun `the 255 the firmware sends for unknown is not a signal strength`() {
        assertNull(rcSignalText(supportsRadio = true, rssi = 255))
    }

    @Test
    fun `zero means no signal and is not shown as a percentage`() {
        assertNull(rcSignalText(supportsRadio = true, rssi = 0))
    }

    @Test
    fun `a real reading is shown as a percentage`() {
        assertEquals("1%", rcSignalText(true, 1))
        assertEquals("72%", rcSignalText(true, 72))
        assertEquals("100%", rcSignalText(true, 100))
    }

    @Test
    fun `anything above 100 is nonsense and hidden`() {
        assertNull(rcSignalText(true, 101))
    }

    @Test
    fun `a vehicle with no radio never shows the cell`() {
        assertNull(rcSignalText(supportsRadio = false, rssi = 72))
    }

    @Test
    fun `a missing reading shows nothing`() {
        assertNull(rcSignalText(true, null))
    }
}

class BatteryLevelTest {
    @Test
    fun `a pack the firmware calls OK is never coloured by percentage`() {
        assertEquals(BatteryLevel.Normal, batteryLevel(1, 5.0, 80, 60))
    }

    @Test
    fun `a critical pack is red even at a high percentage`() {
        assertEquals(BatteryLevel.Critical, batteryLevel(3, 95.0, 80, 60))
    }

    @Test
    fun `emergency failed and unhealthy are all critical`() {
        listOf(4, 5, 6).forEach { state ->
            assertEquals(BatteryLevel.Critical, batteryLevel(state, 95.0, 80, 60))
        }
    }

    @Test
    fun `a low pack warns`() {
        assertEquals(BatteryLevel.Warning, batteryLevel(2, 95.0, 80, 60))
    }

    @Test
    fun `thresholds decide only when the firmware reports no charge state`() {
        assertEquals(BatteryLevel.Normal, batteryLevel(0, 90.0, 80, 60))
        assertEquals(BatteryLevel.Caution, batteryLevel(0, 70.0, 80, 60))
        assertEquals(BatteryLevel.Warning, batteryLevel(0, 50.0, 80, 60))
    }

    @Test
    fun `the user's own thresholds are honoured, not hardcoded ones`() {
        assertEquals(BatteryLevel.Normal, batteryLevel(0, 30.0, 25, 15))
        assertEquals(BatteryLevel.Caution, batteryLevel(0, 20.0, 25, 15))
        assertEquals(BatteryLevel.Warning, batteryLevel(0, 10.0, 25, 15))
    }

    @Test
    fun `an unknown percentage is not treated as empty`() {
        assertEquals(BatteryLevel.Normal, batteryLevel(0, Double.NaN, 80, 60))
        assertEquals(BatteryLevel.Normal, batteryLevel(0, null, 80, 60))
    }
}

class BatteryTextTest {
    @Test
    fun `percentage and voltage both carry their units`() {
        assertEquals(
            "47% · 12.60 V",
            batteryText(47.0, "47", "%", "12.60", " V"),
        )
    }

    @Test
    fun `a nearly full pack reads as full, the way QGC rounds it`() {
        assertEquals("100%", batteryText(99.4, "99.4", "%", null, ""))
    }

    @Test
    fun `voltage alone is shown when percentage is unknown`() {
        assertEquals("12.60 V", batteryText(Double.NaN, "", "%", "12.60", " V"))
    }

    @Test
    fun `a battery reporting nothing shows nothing`() {
        assertNull(batteryText(Double.NaN, null, "%", null, " V"))
    }
}
