package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GuidedSpeedTest {

    @Test
    fun `the core picks the command so the head never chooses airspeed or ground speed`() {
        val multirotor = guidedSpeed(
            JSONObject("""{"available":true,"label":"Ground speed","unit":"m/s",
                "command":"guidedModeChangeGroundSpeedMetersSecond",
                "initial":5.0,"minimum":1.0,"maximum":20.0}"""),
        )!!
        val forward = guidedSpeed(
            JSONObject("""{"available":true,"label":"Airspeed","unit":"m/s",
                "command":"guidedModeChangeEquivalentAirspeedMetersSecond",
                "initial":15.0,"minimum":5.0,"maximum":30.0}"""),
        )!!

        assertEquals("guidedModeChangeGroundSpeedMetersSecond", multirotor.command)
        assertEquals("guidedModeChangeEquivalentAirspeedMetersSecond", forward.command)
        assertEquals("Ground speed", multirotor.label)
        assertEquals("Airspeed", forward.label)
    }

    @Test
    fun `a vehicle with no speed command offers no dialog`() {
        val none = guidedSpeed(
            JSONObject("""{"available":true,"label":null,"unit":"m/s","command":null,
                "initial":null,"minimum":null,"maximum":null}"""),
        )!!

        assertNull(none.command)
        assertEquals("Speed", none.label)
        assertFalse(speedRangeUsable(none))
    }

    @Test
    fun `a usable range needs a command and both bounds`() {
        assertTrue(
            speedRangeUsable(
                guidedSpeed(
                    JSONObject("""{"available":true,"command":"guidedModeChangeGroundSpeedMetersSecond",
                        "label":"Ground speed","unit":"m/s","minimum":1.0,"maximum":20.0,"initial":5.0}"""),
                ),
            ),
        )
        assertFalse(speedRangeUsable(null))
        assertNull(guidedSpeed(JSONObject("""{"available":false}""")))
    }

    @Test
    fun `the argument path carries the target and no separators`() {
        assertEquals("view.guidedSpeed(7.50)", guidedSpeedPath(7.5))
        assertFalse(guidedSpeedPath(7.5).contains(","))
    }
}
