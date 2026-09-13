package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AltitudeEntryTest {
    @Test
    fun `a plain number is an altitude`() {
        assertEquals(47.0, parsedAltitude("47")!!, 1e-9)
        assertEquals(47.5, parsedAltitude(" 47.5 ")!!, 1e-9)
        assertEquals(0.0, parsedAltitude("0")!!, 1e-9)
    }

    @Test
    fun `nonsense and half typed values are refused rather than sent`() {
        assertNull(parsedAltitude(""))
        assertNull(parsedAltitude("."))
        assertNull(parsedAltitude("4x"))
    }

    @Test
    fun `below the ground is not an altitude`() {
        assertNull(parsedAltitude("-5"))
    }

    @Test
    fun `the field starts from the altitude the item has`() {
        assertEquals("75", altitudeFieldText(75.0))
        assertEquals("", altitudeFieldText(Double.NaN))
    }
}
