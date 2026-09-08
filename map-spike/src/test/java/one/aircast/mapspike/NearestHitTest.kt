package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class NearestHitTest {
    @Test
    fun `the closest point wins rather than the first drawn`() {
        val points = listOf(100f to 100f, 12f to 12f, 60f to 60f)

        assertEquals(1, nearestIndex(10f, 10f, points))
    }

    @Test
    fun `a tie goes to the one drawn first`() {
        val points = listOf(20f to 10f, 0f to 10f)

        assertEquals(0, nearestIndex(10f, 10f, points))
    }

    @Test
    fun `something that did not project sorts last`() {
        val points = listOf(null, 40f to 40f)

        assertEquals(1, nearestIndex(10f, 10f, points))
    }

    @Test
    fun `but still wins when it is all there is`() {
        assertEquals(0, nearestIndex(10f, 10f, listOf(null, null)))
    }

    @Test
    fun `nothing under the tap is no hit`() {
        assertNull(nearestIndex(10f, 10f, emptyList()))
    }
}
