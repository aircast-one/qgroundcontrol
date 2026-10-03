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

    @Test
    fun `a circle's centre is edited as its shape centre, like Edit Center Position`() {
        val (hit, at) = shapeCentreHit(ShapeTarget(fencePath(0), line = false), listOf(fence), emptyList())!!
        assertEquals(MapHit.ShapeCentre(fence = true, owner = 0), hit)
        assertEquals(centre.latitude, at.latitude, 1e-6)
        assertEquals(centre.longitude, at.longitude, 1e-6)
        assertEquals("Edit center position", positionTitle(hit))
        assertEquals("Edit vertex position", positionTitle(MapHit.FenceVertex(0, 1)))
        assertNull(shapeCentreHit(ShapeTarget(fencePath(3), line = false), listOf(fence), emptyList()))
    }
}

class DragStepTest {

    @Test
    fun `a drag step dispatched before the drop never lands after it`() {
        val before = moveGeneration()
        writeMove(MapHit.Midpoint("", "", 0), 0.0, 0.0, emptyList(), emptyList())
        assertEquals(true, writeDragStep(before, MapHit.Midpoint("", "", 0), 0.0, 0.0, emptyList(), emptyList(), emptyList(), emptyList()))
        assertEquals("a current step is written, and a midpoint write reports false", false, writeDragStep(moveGeneration(), MapHit.Midpoint("", "", 0), 0.0, 0.0, emptyList(), emptyList(), emptyList(), emptyList()))
    }
}

class LiveCircleTest {

    @Test
    fun `circle mode follows the shape at that path, not just its index`() {
        val ring = circleAround(circleRing(TrackPoint(47.0, 8.0), 100.0, 16), 100.0)!!
        val square = listOf(TrackPoint(47.0, 8.0), TrackPoint(47.001, 8.0), TrackPoint(47.001, 8.001), TrackPoint(47.0, 8.001))
        val chosen = setOf(fencePath(0))
        assertEquals(chosen, liveCircles(chosen, listOf(FencePolygon(0, true, ring)), emptyList()))
        assertEquals("fence 0 was deleted and a square slid into its place", emptySet<String>(), liveCircles(chosen, listOf(FencePolygon(0, true, square)), emptyList()))
        assertEquals(emptySet<String>(), liveCircles(chosen, emptyList(), emptyList()))
    }
}
