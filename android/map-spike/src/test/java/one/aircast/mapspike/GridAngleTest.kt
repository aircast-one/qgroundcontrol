package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class GridAngleTest {
    @Test
    fun `a grid with no angle yet starts from zero`() {
        assertEquals(30.0, nextGridAngle(Double.NaN), 1e-9)
    }

    @Test
    fun `each step turns the grid by thirty degrees`() {
        assertEquals(60.0, nextGridAngle(30.0), 1e-9)
        assertEquals(90.0, nextGridAngle(60.0), 1e-9)
    }

    @Test
    fun `the last step of a turn comes back to zero rather than to 360`() {
        assertEquals(0.0, nextGridAngle(330.0), 1e-9)
    }

    @Test
    fun `twelve steps is one whole turn`() {
        val turned = (1..12).fold(0.0) { angle, _ -> nextGridAngle(angle) }

        assertEquals(0.0, turned, 1e-9)
    }
}
