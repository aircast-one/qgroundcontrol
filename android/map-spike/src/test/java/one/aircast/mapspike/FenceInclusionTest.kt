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
    fun `a circle is named by its edge or its centre and only described, like GeoFenceEditor`() {
        val circles = listOf(circle(2, false))

        assertEquals(false, selectedFence(MapHit.Circle(2), emptyList(), circles)?.keepsIn)
        assertEquals(false, selectedFence(MapHit.CircleCentre(2), emptyList(), circles)?.keepsIn)
        assertEquals(null, selectedFence(MapHit.Circle(2), emptyList(), circles)?.flip)
        assertEquals(true, selectedFence(MapHit.FenceVertex(0, 0), listOf(polygon(0, true)), emptyList())?.flip != null)
    }

    @Test
    fun `a selection that is not a fence offers no flip`() {
        assertNull(selectedFence(MapHit.Waypoint(0), listOf(polygon(0, true)), emptyList()))
        assertNull(selectedFence(null, listOf(polygon(0, true)), emptyList()))
        assertNull(selectedFence(MapHit.FenceVertex(9, 0), listOf(polygon(0, true)), emptyList()))
    }

    @Test
    fun `a new fence is sized from the visible map, like GeoFenceEditor's viewport corners`() {
        val view = listOf(TrackPoint(48.0, 8.0), TrackPoint(48.0, 9.0), TrackPoint(47.0, 9.0), TrackPoint(47.0, 8.0))
        assertEquals(TrackPoint(48.0, 8.0) to TrackPoint(47.0, 9.0), fenceWindow(view, TrackPoint(47.5, 8.5)))
        assertEquals(TrackPoint(47.502, 8.498) to TrackPoint(47.498, 8.502), fenceWindow(emptyList(), TrackPoint(47.5, 8.5)))
    }
}
