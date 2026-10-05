package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class GuidedValueFlowTest {
    private val kind = GuidedValueKind("takeoff", "Takeoff", null, "Take off", "no range", quickPicks = false, read = { null }, commit = {})

    @Test
    fun `a value opens only with a usable range and a starting value`() {
        val usable = GuidedReading("Height", "m", 1.0..120.0, 10.0, "", sendable = true)
        assertNotNull(openedGuidedValue(kind, usable))
        assertEquals(10.0, openedGuidedValue(kind, usable)!!.target, 1e-9)
        assertNull(openedGuidedValue(kind, usable.copy(range = null)))
        assertNull(openedGuidedValue(kind, usable.copy(initial = null)))
        assertNull(openedGuidedValue(kind, null))
    }

    @Test
    fun `each guided reading maps onto the shared shape`() {
        val takeoff = takeoffReading(GuidedTakeoff("Height", "ft", 10.0, 10.0, 400.0, "climb", 3.0))!!
        assertEquals(10.0..400.0, takeoff.range)
        assertEquals(10.0, takeoff.initial)
        assertNull(takeoffReading(GuidedTakeoff("Height", "ft", 10.0, 400.0, 10.0, "", 3.0))!!.range)

        val altitude = altitudeReading(GuidedAltitude(true, "Alt", "m", 25.0, 1.0, 120.0, "", 0.0, sends = false))!!
        assertEquals(25.0, altitude.initial)
        assertFalse(altitude.sendable)
    }
}
