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
    fun `orbit at location opens a default circle on the tapped point that the radius field edits in app units`() {
        val feet = orbitDefaults(JSONObject("""{"orbitDefaultRadius":98.4252,"orbitRadiusUnit":"ft","orbitMetresPerUnit":0.3048,"orbitClockwise":true}"""))
        val opened = orbitOpened(MapPoint(47.4, 8.5), feet)
        assertEquals(one.aircast.mapspike.TrackPoint(47.4, 8.5), opened.centre)
        assertEquals(30.0, opened.radiusMetres, 1e-3)
        assertTrue(opened.clockwise)
        val edit = orbitEdit(opened.copy(radiusMetres = 15.24, clockwise = false), feet)
        assertEquals("50", loiterRadiusField(null, edit))
        assertEquals(30.48, loiterTyped("100", edit).radiusMetres, 1e-9)
        assertEquals(one.aircast.mapspike.MINIMUM_CIRCLE_RADIUS_METRES, orbitOpened(MapPoint(47.4, 8.5), orbitDefaults(null)).radiusMetres, 0.0)
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

    @Test
    fun `a loiter change re-sends the goto point with the direction as the sign`() {
        val offer = loiterOffer(JSONObject("""{"loiter":{"latitude":47.4,"longitude":8.5,"title":"Change Loiter Radius","message":"m","defaultRadius":80,"clockwise":true}}"""))!!
        assertEquals(47.4, offer.latitude, 0.0)
        assertEquals(80.0, offer.defaultRadius, 0.0)
        assertEquals(-80.0, signedLoiterRadius(80.0, clockwise = false), 0.0)
        assertEquals(80.0, signedLoiterRadius(-80.0, clockwise = true), 0.0)
        assertEquals(null, loiterOffer(JSONObject("""{"loiter":null}""")))
    }

    @Test
    fun `the loiter radius field follows the map drag but keeps what the operator is typing`() {
        val offer = loiterOffer(JSONObject("""{"loiter":{"latitude":47.4,"longitude":8.5,"title":"t","message":"m","defaultRadius":250,"clockwise":false}}"""))!!
        val opened = loiterEditOpened(offer, OrbitDefaults(0.0, "ft", 0.3048, true))
        assertEquals(76.2, opened.radiusMetres, 1e-9)
        assertFalse(opened.clockwise)
        assertEquals("250", loiterRadiusField(null, opened))
        assertEquals("250.", loiterRadiusField("250.", opened))
        assertEquals("", loiterRadiusField("", opened))
        val dragged = opened.copy(radiusMetres = 100 * 0.3048)
        assertEquals("100", loiterRadiusField("250", dragged))
        assertEquals(30.48, loiterTyped("100", opened).radiusMetres, 1e-9)
        assertEquals(opened, loiterTyped("abc", opened))
        assertEquals(opened, loiterTyped("0", opened))
    }

    @Test
    fun `go here reads how far and which way the point lies, in the operator's unit`() {
        val from = MapPoint(47.0, 8.0)
        assertEquals("111 m north", goHereText(from, MapPoint(47.001, 8.0), "m", 1.0))
        assertEquals("365 ft north", goHereText(from, MapPoint(47.001, 8.0), "ft", 0.3048))
        assertEquals("76 m east", goHereText(from, MapPoint(47.0, 8.001), "m", 1.0))
        assertEquals(null, goHereText(from, MapPoint(47.001, 8.0), "", 1.0))
    }
}
