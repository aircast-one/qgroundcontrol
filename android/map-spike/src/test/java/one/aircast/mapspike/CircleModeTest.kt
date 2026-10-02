package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CircleModeTest {

    private val centre = TrackPoint(47.0, 8.0)
    private val ring = circleRing(centre, 100.0, 36)
    private val fence = FencePolygon(index = 0, inclusion = true, vertices = ring)

    @Test
    fun `a circle's radius is the distance from its centre to a vertex`() {
        assertEquals(100.0, circleRadius(ring)!!, 0.5)
        assertNull(circleRadius(emptyList()))
    }

    @Test
    fun `setting the radius redraws the ring around the same centre`() {
        val wider = circleAround(ring, 250.0)!!
        assertEquals(250.0, circleRadius(wider)!!, 0.5)
        assertEquals(centre.latitude, polygonCentre(wider)!!.latitude, 1e-6)
        assertNull("a zero radius is not a circle", circleAround(ring, 0.0))
    }

    @Test
    fun `only a circled shape gets a radius handle, and loses its corners`() {
        assertEquals(0, radiusHandleFeatures(listOf(fence), emptyList(), emptySet()).size)
        assertEquals(1, radiusHandleFeatures(listOf(fence), emptyList(), setOf(fencePath(0))).size)
    }

    @Test
    fun `a target finds its own vertices`() {
        assertEquals(ring, shapeVertices(ShapeTarget(fencePath(0), line = false), listOf(fence), emptyList()))
        assertEquals(emptyList<TrackPoint>(), shapeVertices(ShapeTarget(fencePath(3), line = false), listOf(fence), emptyList()))
    }
}
