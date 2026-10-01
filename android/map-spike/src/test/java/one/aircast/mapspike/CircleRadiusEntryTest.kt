package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CircleRadiusEntryTest {
    private val circle = FenceCircle(0, true, TrackPoint(47.0, 8.0), 50.0, radiusMinimum = 10.0, radiusMaximum = 500.0)

    @Test
    fun `a typed radius is taken only inside the fence's bounds`() {
        assertEquals(120.5, typedRadius(" 120.5 ", circle)!!, 0.0)
        assertNull(typedRadius("5", circle))
        assertNull(typedRadius("900", circle))
        assertNull(typedRadius("wide", circle))
        assertEquals("120.5", trimmedRadius(120.5))
        assertEquals("50", trimmedRadius(50.0))
    }
}
