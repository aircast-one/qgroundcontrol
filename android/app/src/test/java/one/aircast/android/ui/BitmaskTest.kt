package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BitmaskTest {
    private fun arming(value: Long) = Fact(
        path = "p", name = "ARMING_CHECK", description = "Arm Checks to Perform", units = "",
        valueString = value.toString(), value = value,
        enumStrings = emptyList(), enumIndex = -1,
        isBool = false, isString = false, readOnly = false,
        bitmaskStrings = listOf("All", "Barometer", "Compass", "GPS lock"),
        bitmaskValues = listOf(1L, 2L, 4L, 8L),
    )

    @Test
    fun `a bitmask is not an enum, so it does not get a single-choice picker`() {
        val fact = arming(0)

        assertTrue(fact.isBitmask)
        assertFalse(fact.isEnum)
    }

    @Test
    fun `no bits set says so instead of showing zero`() {
        assertEquals("None", bitmaskSummary(arming(0)))
    }

    @Test
    fun `the set checks are named, which is the whole point`() {
        assertEquals("Barometer, GPS lock", bitmaskSummary(arming(2L or 8L)))
    }

    @Test
    fun `every bit set reads as All rather than a list of everything`() {
        assertEquals("All", bitmaskSummary(arming(1L or 2L or 4L or 8L)))
    }

    @Test
    fun `a value arriving as a string is still read`() {
        val asText = arming(6).copy(value = null, valueString = "6.000")

        assertEquals("Barometer, Compass", bitmaskSummary(asText))
    }

    @Test
    fun `toggling a bit flips only that bit`() {
        val fact = arming(2L or 8L)
        val raw = bitmaskRaw(fact)

        assertEquals(2L or 4L or 8L, raw xor 4L)
        assertEquals(8L, raw xor 2L)
    }

    @Test
    fun `a fact whose bit names and values disagree is not treated as a bitmask`() {
        val ragged = arming(0).copy(bitmaskValues = listOf(1L, 2L))

        assertFalse(ragged.isBitmask)
    }
}
