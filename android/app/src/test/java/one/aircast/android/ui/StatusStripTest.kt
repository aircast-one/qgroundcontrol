package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.json.JSONObject
import org.junit.Test

class RcSignalTest {
    @Test
    fun `the 255 the firmware sends for unknown is not a signal strength`() {
        assertNull(rcSignalText(supportsRadio = true, rssi = 255))
    }

    @Test
    fun `zero is a reading the vehicle chose to send and says the link is dead`() {
        assertEquals("No signal", rcSignalText(supportsRadio = true, rssi = 0))
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






}

class BatteryTextTest {




    @Test
    fun `the level name maps to the ladder this strip already drew`() {
        assertEquals(BatteryLevel.Normal, batteryLevelOf("normal"))
        assertEquals(BatteryLevel.Caution, batteryLevelOf("caution"))
        assertEquals(BatteryLevel.Warning, batteryLevelOf("warning"))
        assertEquals(BatteryLevel.Critical, batteryLevelOf("critical"))
    }

    @Test
    fun `an unknown or absent level is not treated as an alarm`() {
        assertEquals(BatteryLevel.Normal, batteryLevelOf(null))
        assertEquals(BatteryLevel.Normal, batteryLevelOf("a level added later"))
    }

    @Test
    fun `the reading is the core's text and level`() {
        val reading = batteryReading(
            JSONObject("""{"available":true,"level":"caution","text":"70%",
                "packs":[{"secondaryText":"11.10 V"}]}"""),
        )!!

        assertEquals("70% · 11.10 V", reading.text)
        assertEquals(BatteryLevel.Caution, reading.level)
    }

    @Test
    fun `no battery is no cell rather than an empty one`() {
        assertNull(batteryReading(null))
        assertNull(batteryReading(JSONObject("""{"available":false}""")))
        assertNull(batteryReading(JSONObject("""{"available":true,"level":"normal","text":""}""")))
    }

    @Test
    fun `a pack with nothing to add leaves the primary alone`() {
        assertEquals(
            "90%",
            batteryReading(
                JSONObject("""{"available":true,"level":"warning","text":"90%",
                    "packs":[{"secondaryText":""}]}"""),
            )!!.text,
        )
        assertEquals(
            "12.4 V",
            batteryReading(
                JSONObject("""{"available":true,"level":"normal","text":"12.4 V",
                    "packs":[{"secondaryText":"12.4 V"}]}"""),
            )!!.text,
        )
    }
}


class GpsFixTest {
    @Test
    fun `a fix type below 2D is no fix at all`() {
        assertEquals(FixLevel.None, fixLevel(0.0))
        assertEquals(FixLevel.None, fixLevel(1.0))
    }

    @Test
    fun `2D is told apart from 3D rather than both passing as a fix`() {
        assertEquals(FixLevel.TwoD, fixLevel(2.0))
        assertEquals(FixLevel.Good, fixLevel(3.0))
        assertEquals(FixLevel.Good, fixLevel(6.0))
    }

    @Test
    fun `an unreadable lock shows nothing rather than claiming a fix`() {
        assertNull(fixLevel(Double.NaN))
    }

    @Test
    fun `a satellite count is not shown as reassurance when there is no fix`() {
        assertEquals("No fix", satsText(FixLevel.None, "11"))
    }

    @Test
    fun `a 2D fix says so next to the count`() {
        assertEquals("11 · 2D only", satsText(FixLevel.TwoD, "11"))
    }

    @Test
    fun `a good fix is just the count`() {
        assertEquals("11", satsText(FixLevel.Good, "11"))
    }

    @Test
    fun `the rendered lock text is not a fix level, so wiring the string back in hides the cell`() {
        assertNull(fixLevel("3D Lock".toDoubleOrNull() ?: Double.NaN))
        assertNull(fixLevel("None".toDoubleOrNull() ?: Double.NaN))
    }
}
