package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
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
            surveys = listOf(Survey(0, listOf(TrackPoint(41.0, 44.0)), listOf(TrackPoint(41.9, 44.9)), 0, KIND_SURVEY, SHAPE_AREA, "surveyAreaPolygon")),
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

class FitPointsTest {
    private fun item(latitude: Double, longitude: Double) =
        MissionItem(0, 1, latitude, longitude, "Waypoint", false, Double.NaN)

    @Test
    fun `a plan is framed on its own terms`() {
        val plan = planPoints(items = listOf(item(41.0, 44.0), item(41.5, 44.5)))

        assertEquals(plan, fitPoints(plan, -35.36, 149.16))
    }

    @Test
    fun `an empty plan falls back to the aircraft`() {
        val points = fitPoints(emptyList(), -35.36, 149.16)

        assertEquals(listOf(TrackPoint(-35.36, 149.16)), points)
    }

    @Test
    fun `with no plan and no position there is nothing to frame`() {
        assertTrue(fitPoints(emptyList(), Double.NaN, Double.NaN).isEmpty())
        assertTrue(fitPoints(emptyList(), 0.0, 0.0).isEmpty())
    }
}

class LongitudeWraparoundTest {
    private fun at(vararg longitudes: Double) =
        planBounds(longitudes.map { TrackPoint(10.0, it) })!!

    @Test
    fun `two points either side of the antimeridian do not frame the planet`() {
        val bounds = at(179.9, -179.9)

        assertEquals(0.2, bounds.longitudeSpan, 1e-9)
        assertEquals(180.0, bounds.centre.longitude, 1e-9)
    }

    @Test
    fun `an ordinary region is unchanged by the arc treatment`() {
        val bounds = at(44.7, 44.9, 44.8)

        assertEquals(44.7, bounds.west, 1e-9)
        assertEquals(44.9, bounds.east, 1e-9)
        assertEquals(0.2, bounds.longitudeSpan, 1e-9)
        assertEquals(44.8, bounds.centre.longitude, 1e-9)
    }

    @Test
    fun `the widest empty gap is the one left out of the arc`() {
        val bounds = at(-10.0, 10.0, 170.0)

        assertEquals(-10.0, bounds.west, 1e-9)
        assertEquals(170.0, bounds.east, 1e-9)
        assertEquals(180.0, bounds.longitudeSpan, 1e-9)
        assertEquals(80.0, bounds.centre.longitude, 1e-9)
    }

    @Test
    fun `a single point spans nothing and centres on itself`() {
        val bounds = at(-179.95)

        assertEquals(0.0, bounds.longitudeSpan, 1e-9)
        assertEquals(-179.95, bounds.centre.longitude, 1e-9)
    }

    @Test
    fun `the span never exceeds the planet`() {
        val bounds = at(-179.0, -90.0, 0.0, 90.0, 179.0)

        assertTrue(bounds.longitudeSpan <= 360.0)
        assertEquals(270.0, bounds.longitudeSpan, 1e-9)
    }

    @Test
    fun `normalising keeps a longitude in range`() {
        assertEquals(-179.0, normaliseLongitude(181.0), 1e-9)
        assertEquals(179.0, normaliseLongitude(-181.0), 1e-9)
        assertEquals(180.0, normaliseLongitude(180.0), 1e-9)
        assertEquals(0.0, normaliseLongitude(720.0), 1e-9)
    }
}

class TakeoffMissingTest {
    private fun item(kind: String, name: String, lat: Double = 41.0, lon: Double = 44.0) =
        MissionItem(0, 1, lat, lon, name, false, 50.0, kind = kind)

    @Test
    fun `waypoints with no takeoff need one inserted first`() {
        assertTrue(takeoffMissing(listOf(item("waypoint", "Waypoint"), item("waypoint", "Waypoint"))))
    }

    @Test
    fun `a plan that already has a takeoff does not get another`() {
        assertFalse(takeoffMissing(listOf(item("takeoff", "Takeoff"), item("waypoint", "Waypoint"))))
    }

    @Test
    fun `a VTOL takeoff counts as a takeoff`() {
        assertFalse(takeoffMissing(listOf(item("takeoff", "VTOL Takeoff"), item("waypoint", "Waypoint"))))
    }

    @Test
    fun `a takeoff named in another language still counts`() {
        assertFalse(takeoffMissing(listOf(item("takeoff", "Starten"), item("waypoint", "Wegpunkt"))))
    }

    @Test
    fun `an empty plan needs nothing inserted before anything`() {
        assertFalse(takeoffMissing(emptyList()))
    }

    @Test
    fun `items with no coordinate do not by themselves require a takeoff`() {
        assertFalse(takeoffMissing(listOf(item("command", "Return To Launch", 0.0, 0.0))))
    }
}
