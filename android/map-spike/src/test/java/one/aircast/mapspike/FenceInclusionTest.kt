package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FenceInclusionTest {

    private fun polygon(index: Int, inclusion: Boolean) = FencePolygon(
        index = index,
        inclusion = inclusion,
        vertices = (0 until 4).map { TrackPoint(41.0 + it, 44.0) },
        kindText = if (inclusion) "Keep-in polygon" else "Keep-out polygon",
    )

    private fun circle(index: Int, inclusion: Boolean) = FenceCircle(
        index = index,
        inclusion = inclusion,
        centre = TrackPoint(41.0, 44.0),
        radius = 150.0,
        kindText = if (inclusion) "Keep-in circle" else "Keep-out circle",
    )

    @Test
    fun `a keep-in fence offers to become keep-out, and the other way`() {
        val keepIn = selectedFence(MapHit.FenceVertex(0, 0), listOf(polygon(0, true)), emptyList())
        val keepOut = selectedFence(MapHit.FenceVertex(0, 0), listOf(polygon(0, false)), emptyList())

        assertEquals(true, keepIn?.keepsIn)
        assertEquals(false, keepOut?.keepsIn)
    }

    @Test
    fun `a circle is flipped whether it was named by its edge or its centre`() {
        val circles = listOf(circle(2, false))

        assertEquals(false, selectedFence(MapHit.Circle(2), emptyList(), circles)?.keepsIn)
        assertEquals(false, selectedFence(MapHit.CircleCentre(2), emptyList(), circles)?.keepsIn)
    }

    @Test
    fun `a selection that is not a fence offers no flip`() {
        assertNull(selectedFence(MapHit.Waypoint(0), listOf(polygon(0, true)), emptyList()))
        assertNull(selectedFence(null, listOf(polygon(0, true)), emptyList()))
        assertNull(selectedFence(MapHit.FenceVertex(9, 0), listOf(polygon(0, true)), emptyList()))
    }
}
