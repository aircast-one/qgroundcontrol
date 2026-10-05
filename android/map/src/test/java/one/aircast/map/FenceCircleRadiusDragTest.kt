package one.aircast.map

import org.junit.Assert.assertEquals
import org.junit.Test

class FenceCircleRadiusDragTest {
    private val centre = TrackPoint(47.0, 8.0)

    @Test
    fun `the edge handle sits east of the centre at the radius, as QGCMapCircleVisuals places its drag handle`() {
        val circle = FenceCircle(3, true, centre, 100.0)
        val edge = circleEdge(circle)
        assertEquals(100.0, metresBetween(centre, edge), 0.5)
        assertEquals(true, edge.longitude > centre.longitude)
        val handle = vertexHandleFeatures(emptyList(), emptyList(), listOf(circle)).features().orEmpty()
            .first { it.getStringProperty(HANDLE_KIND_PROPERTY) == HANDLE_KIND_FENCE_CIRCLE_RADIUS }
        assertEquals(MapHit.CircleRadius(3), handleHit(handle.getStringProperty(HANDLE_KIND_PROPERTY), handle.getNumberProperty(POLYGON_INDEX_PROPERTY).toInt(), handle.getNumberProperty(VERTEX_INDEX_PROPERTY).toInt()))
    }

    @Test
    fun `dragging sets the distance from the centre, in the units the radius is shown in`() {
        val feet = FenceCircle(0, true, centre, 328.084, radiusMetres = 100.0, radiusUnits = "ft")
        val to = pointAt(centre, 200.0, 90.0)
        assertEquals(656.168, draggedCircleRadius(feet, to), 0.5)
    }

    @Test
    fun `a drag past the fence limits stops at them`() {
        val bounded = FenceCircle(0, true, centre, 100.0, radiusMinimum = 10.0, radiusMaximum = 500.0)
        assertEquals(10.0, draggedCircleRadius(bounded, pointAt(centre, 1.0, 90.0)), 1e-9)
        assertEquals(500.0, draggedCircleRadius(bounded, pointAt(centre, 900.0, 90.0)), 1e-9)
        val unbounded = FenceCircle(0, true, centre, 100.0)
        assertEquals("a drag onto the centre keeps QGCMapCircle's 0.1 m so the circle is not dropped", 0.1, draggedCircleRadius(unbounded, centre), 1e-9)
    }
}
