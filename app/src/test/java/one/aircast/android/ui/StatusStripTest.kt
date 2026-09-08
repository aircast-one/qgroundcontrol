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
