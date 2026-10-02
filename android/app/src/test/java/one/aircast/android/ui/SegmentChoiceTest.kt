package one.aircast.android.ui

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SegmentChoiceTest {
    @Test
    fun shortWritableEnumsBecomeSegments() =
        assertTrue(showsAsSegments(isEnum = true, offList = false, writable = true, options = listOf("Indoor", "Outdoor", "System")))

    @Test
    fun longOrManyOptionsStayADropdown() {
        assertFalse(showsAsSegments(true, false, true, listOf("Feet/Second", "Meters/Second", "Miles/Hour", "Knots", "Km/Hour")))
        assertFalse(showsAsSegments(true, false, true, listOf("A very long first option", "And a second one")))
    }

    @Test
    fun readOnlyOrOffListValuesStayAsTheyAre() {
        assertFalse(showsAsSegments(true, false, false, listOf("On", "Off")))
        assertFalse(showsAsSegments(true, true, true, listOf("On", "Off")))
        assertFalse(showsAsSegments(false, false, true, emptyList()))
    }
}
