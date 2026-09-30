package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class MapClickMenuTest {
    @Test
    fun `reads the offered actions in the order the core serves them`() {
        val actions = mapClickActions(
            JSONObject(
                """{"actions":[{"id":"GoTo","path":"vehicle.guidedModeGotoLocation","label":"Go to location","title":"Go To Location",""" +
                    """"message":"Move the vehicle to the specified location","confirm":true},""" +
                    """{"path":"vehicle.guidedModeROI","label":"ROI at location","title":"ROI","message":"m","confirm":false}]}""",
            ),
        )
        assertEquals(listOf("Go to location", "ROI at location"), actions.map { it.label })
        assertTrue(actions[0].confirm)
        assertFalse(actions[1].confirm)
        assertTrue(mapClickActions(null).isEmpty())
    }

    @Test
    fun `an orbit sends the point, radius, direction and height above home in that order`() {
        val args = orbitArgs(MapPoint(47.4, 8.5), OrbitChoice(radiusMetres = 30.0, clockwise = false, aboveHomeMetres = 50.0))
        assertEquals(listOf<Any>(47.4, 8.5, 30.0, false, 50.0), args.toList())
        val feet = orbitDefaults(JSONObject("""{"orbitDefaultRadius":98.4252,"orbitRadiusUnit":"ft","orbitMetresPerUnit":0.3048,"orbitClockwise":true}"""))
        assertEquals(30.0, radiusMetres("", feet)!!, 1e-3)
        assertEquals(15.24, radiusMetres("50", feet)!!, 1e-9)
        assertEquals(null, radiusMetres("wide", feet))
    }

    @Test
    fun `shows the point to six places as QGC does`() {
        assertEquals(listOf("Lat: 47.397000", "Lon: 8.545123"), coordinateLines(MapPoint(47.397, 8.5451234)))
    }

    @Test
    fun `a tapped item jumps to its sequence, never before the first waypoint`() {
        assertEquals(1, waypointTarget(0))
        assertEquals(4, waypointTarget(4))
        assertEquals("Adjust current waypoint to 4", setWaypointMessage(4))
    }
}
