package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PlanBoundsTest {
    private fun item(latitude: Double, longitude: Double) =
        MissionItem(0, 1, latitude, longitude, "Waypoint", false, Double.NaN)

    @Test
    fun `bounds cover every corner of the plan`() {
        val bounds = planBounds(
            planPoints(items = listOf(item(41.0, 44.0), item(41.5, 44.5), item(41.2, 43.8))),
        )!!

        assertEquals(41.0, bounds.south, 1e-9)
        assertEquals(43.8, bounds.west, 1e-9)
        assertEquals(41.5, bounds.north, 1e-9)
        assertEquals(44.5, bounds.east, 1e-9)
    }

    @Test
    fun `a circle contributes its ring so a fit does not cut it in half`() {
        val centre = TrackPoint(41.0, 44.0)
        val circle = FenceCircle(0, true, centre, 500.0)

        val bounds = planBounds(planPoints(circles = listOf(circle)))!!

        assertTrue(bounds.north > centre.latitude)
        assertTrue(bounds.south < centre.latitude)
        assertTrue(bounds.east > centre.longitude)
        assertTrue(bounds.west < centre.longitude)
    }

    @Test
    fun `fences rally points and surveys all count`() {
        val points = planPoints(
            polygons = listOf(FencePolygon(0, true, listOf(TrackPoint(40.0, 43.0)))),
            rally = listOf(RallyPoint(0, 42.0, 45.0)),
            surveys = listOf(Survey(0, listOf(TrackPoint(41.0, 44.0)), listOf(TrackPoint(41.9, 44.9)), 0)),
        )

        val bounds = planBounds(points)!!

        assertEquals(40.0, bounds.south, 1e-9)
        assertEquals(43.0, bounds.west, 1e-9)
        assertEquals(42.0, bounds.north, 1e-9)
        assertEquals(45.0, bounds.east, 1e-9)
    }

    @Test
    fun `an empty or unplottable plan has no bounds to fit`() {
        assertNull(planBounds(emptyList()))
        assertNull(planBounds(listOf(TrackPoint(0.0, 0.0))))
    }

    @Test
    fun `a single point still yields bounds centred on itself`() {
        val bounds = planBounds(listOf(TrackPoint(41.0, 44.0)))!!

        assertEquals(TrackPoint(41.0, 44.0), bounds.centre)
        assertEquals(0.0, bounds.spanDegrees, 1e-9)
    }
}
