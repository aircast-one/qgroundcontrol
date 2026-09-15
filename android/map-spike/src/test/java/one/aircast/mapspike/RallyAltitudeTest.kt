package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class RallyAltitudeTest {

    private val served = JSONObject(
        """{"kind":"object","class":"Fences","rallyPoints":[
             {"index":0,"latitude":47.397,"longitude":8.545,"altitudeMetres":50.0},
             {"index":1,"latitude":47.398,"longitude":8.546,"altitudeMetres":120.0}]}""",
    )

    @Test
    fun `a rally point carries the height the bridge will write back`() {
        val points = rallyPoints(served)

        assertEquals(
            "RallyPoint::setCoordinate writes coordinate.altitude() into _altitudeFact - unlike " +
                "SimpleMissionItem::setCoordinate, which takes lat/lon only - so dragging one with " +
                "no altitude sets a failsafe destination to ground level",
            50.0,
            points[0].altitudeMetres,
            1e-9,
        )
        assertEquals(120.0, points[1].altitudeMetres, 1e-9)
    }

    @Test
    fun `a core too old to serve the height writes zero rather than crashing`() {
        val old = JSONObject(
            """{"kind":"object","class":"Fences","rallyPoints":[
                 {"index":0,"latitude":47.397,"longitude":8.545}]}""",
        )

        assertEquals(0.0, rallyPoints(old)[0].altitudeMetres, 1e-9)
    }

    @Test
    fun `the coordinate written carries the altitude, in metres`() {
        val json = coordinateJson(47.397, 8.545, 50.0)

        assertTrue("\"altitude\":50.0 is what the bridge reads as metres", json.contains("\"altitude\":50.0"))
        assertTrue(json.contains("\"latitude\":47.397"))
    }

    @Test
    fun `the payload a drag writes keeps the point's own height`() {
        assertEquals(
            """{"value":{"latitude":47.398,"longitude":8.546,"altitude":120.0}}""",
            rallyMovePayload(47.398, 8.546, 120.0),
        )
    }

    @Test
    fun `the height comes from the point being dragged, not the first one`() {
        val points = rallyPoints(served)

        assertEquals(120.0, rallyAltitudeFor(points, 1), 1e-9)
        assertEquals(50.0, rallyAltitudeFor(points, 0), 1e-9)
        assertEquals(
            "an index with no point is the only case that may write zero, and it cannot move " +
                "a point that is not there",
            0.0,
            rallyAltitudeFor(points, 7),
            1e-9,
        )
    }

    @Test
    fun `a caller that does not care still gets the old zero`() {
        assertTrue(coordinateJson(47.397, 8.545).contains("\"altitude\":0.0"))
    }
}
