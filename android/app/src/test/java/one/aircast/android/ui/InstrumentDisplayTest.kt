package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class InstrumentDisplayTest {
    private val ranged = ValueDisplay(colourRange = true, values = listOf(10.0, 20.0), colours = listOf(1L, 2L, 3L))

    @Test
    fun `a value takes the colour of the first range it fits under`() {
        assertEquals(1L, displayColour(ranged, 5.0))
        assertEquals(2L, displayColour(ranged, 20.0))
        assertEquals(3L, displayColour(ranged, 25.0))
        assertEquals("an unknown value is the first range, as QGC treats NaN", 1L, displayColour(ranged, null))
        assertNull(displayColour(ValueDisplay(), 5.0))
    }

    @Test
    fun `turning on a colour range starts from QGC's two green thresholds`() {
        val on = withColourRange(ValueDisplay(), true)
        assertEquals(listOf(0.0, 100.0), on.values)
        assertEquals(3, on.colours.size)
        assertEquals(3, withRow(on).values.size)
        assertEquals(listOf(100.0), withoutRow(on, 0).values)
        assertEquals(ValueDisplay(), withColourRange(on, false))
    }

    @Test
    fun `units can be hidden and the display survives a round trip`() {
        assertEquals("12.1", displayReading(ValueDisplay(showUnits = false), "12.1", "V"))
        assertEquals("12.1 V", displayReading(ValueDisplay(), "12.1", "V"))
        val custom = ranged.copy(text = "Pack", showUnits = false)
        assertEquals(custom, displayFrom(displayJson(custom)))
        assertEquals(ValueDisplay(), displayFrom("not json"))
    }
}
