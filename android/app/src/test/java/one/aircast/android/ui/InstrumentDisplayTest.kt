package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class InstrumentDisplayTest {
    private val ranged = ValueDisplay(rangeType = RangeType.Color, values = listOf(10.0, 20.0), colours = listOf(1L, 2L, 3L))

    @Test
    fun `a value takes the colour of the range it falls in`() {
        assertEquals(1L, displayColour(ranged, 5.0))
        assertEquals(2L, displayColour(ranged, 20.0))
        assertEquals(3L, displayColour(ranged, 25.0))
        assertEquals("an unknown value is the first range, as QGC treats NaN", 1L, displayColour(ranged, null))
        assertNull(displayColour(ValueDisplay(), 5.0))
        assertNull("an unchecked colour slot falls back to the palette colour", displayColour(ranged.copy(colours = listOf(NO_COLOUR, 2L, 3L)), -1.0))
    }

    @Test
    fun `forward flight vehicles keep airspeed among their default values like QGCCorePlugin`() {
        assertEquals(DEFAULT_INSTRUMENTS + "airSpeed", defaultInstruments("fixedWing"))
        assertEquals(DEFAULT_INSTRUMENTS, defaultInstruments("multiRotor"))
    }

    @Test
    fun `removing a threshold drops the band above it, as InstrumentValueData removeRangeValue does`() {
        val coloured = ValueDisplay(rangeType = RangeType.Color, values = listOf(0.0, 50.0), colours = listOf(1L, 2L, 3L))
        assertEquals(listOf(1L, 3L), withoutRow(coloured, 0).colours)
    }

    @Test
    fun `switching the range type resets the rows like _resetRangeInfo`() {
        val icons = withRangeType(ValueDisplay(), RangeType.Icon, "airplane.svg")
        assertEquals(listOf(0.0, 100.0), icons.values)
        assertEquals(List(3) { "airplane.svg" }, icons.icons)
        assertTrue(icons.colours.isEmpty())
        assertEquals(3, withRow(icons, "x.svg").values.size)
        assertEquals("x.svg", withRow(icons, "x.svg").icons.last())
        assertEquals(listOf(100.0), withoutRow(icons, 0).values)
        assertEquals(ValueDisplay(), withRangeType(icons, RangeType.None, "airplane.svg"))
    }

    @Test
    fun `opacity and icon ranges pick by value, and a fixed icon replaces the label`() {
        val faded = withRangeType(ValueDisplay(), RangeType.Opacity, "a.svg").let { it.copy(opacities = listOf(0.2, 0.5, 1.0)) }
        assertEquals(0.2f, displayOpacity(faded, -1.0))
        assertEquals(1f, displayOpacity(faded, 150.0))
        assertEquals(1f, displayOpacity(ValueDisplay(), 5.0))
        val swapped = withRangeType(ValueDisplay(), RangeType.Icon, "a.svg").let { it.copy(icons = listOf("low.svg", "mid.svg", "high.svg")) }
        assertEquals("mid.svg", displayIcon(swapped, 50.0))
        assertEquals("plane.svg", displayIcon(ValueDisplay(showIcon = true, icon = "plane.svg"), 5.0))
        assertNull(displayIcon(ValueDisplay(), 5.0))
    }

    @Test
    fun `a display survives a round trip through storage`() {
        val display = ValueDisplay(text = "Alt", showIcon = true, icon = "plane.svg", rangeType = RangeType.Opacity, values = listOf(1.0), opacities = listOf(0.5, 1.0))
        assertEquals(display, displayFrom(displayJson(display)))
    }
}
