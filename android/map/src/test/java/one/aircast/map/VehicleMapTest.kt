package one.aircast.map

import org.junit.Assert.assertEquals
import org.junit.Test

class VehicleMapTest {
    @Test
    fun `a map still showing the whole world lands on its plan at once instead of flying in through continent scales`() {
        assertEquals(false, placedBefore(0.0))
        assertEquals("a map already placed keeps its smooth moves", true, placedBefore(15.0))
    }
}
