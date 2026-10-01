package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class IndicatorOrderTest {
    @Test
    fun `saved order comes first and new indicators follow in their own order`() {
        assertEquals(listOf("gps", "battery", "rc"), orderedKeys(listOf("battery", "gps", "rc"), listOf("gps", "gone", "battery")))
    }

    @Test
    fun `an indicator moves one slot and not past either end`() {
        assertEquals(listOf("gps", "battery", "rc"), movedKey(listOf("battery", "gps", "rc"), "battery", 1))
        assertEquals(listOf("battery", "rc", "gps"), movedKey(listOf("battery", "gps", "rc"), "rc", -1))
        assertNull(movedKey(listOf("battery", "gps"), "battery", -1))
        assertNull(movedKey(listOf("battery", "gps"), "nope", 1))
    }
}
