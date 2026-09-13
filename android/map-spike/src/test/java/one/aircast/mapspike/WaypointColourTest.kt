package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class WaypointColourTest {
    @Test
    fun `the commands that change what a plan means each get their own colour`() {
        assertEquals(TAKEOFF_COLOUR, waypointColour("takeoff", 22))
        assertEquals(LAND_COLOUR, waypointColour("land", 21))
        assertEquals(RETURN_COLOUR, waypointColour("command", MAV_CMD_NAV_RETURN_TO_LAUNCH))
        assertEquals(LOITER_COLOUR, waypointColour("command", MAV_CMD_NAV_LOITER_TIME))
    }

    @Test
    fun `the launch point is not drawn as somewhere the aircraft flies to`() {
        assertEquals(START_COLOUR, waypointColour("settings", 16))
    }

    @Test
    fun `an ordinary waypoint keeps the plain colour`() {
        assertEquals(WAYPOINT_COLOUR, waypointColour("waypoint", 16))
        assertEquals(WAYPOINT_COLOUR, waypointColour("", 0))
    }

    @Test
    fun `a VTOL takeoff is a takeoff, because the kind says so and the name need not`() {
        assertEquals(TAKEOFF_COLOUR, waypointColour("takeoff", 84))
        assertEquals(LAND_COLOUR, waypointColour("land", 85))
    }

    @Test
    fun `a plan in another language is drawn the same, because no colour reads a name`() {
        val german = JSONObject(
            """{"items":[{"kind":"takeoff","command":22,"name":"Starten",""" +
                """"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0}},""" +
                """{"kind":"command","command":20,"name":"Rückkehr zum Start",""" +
                """"flownLeg":true,"coordinate":{"latitude":41.1,"longitude":44.1}}]}""",
        )
        val items = missionItems(german)

        assertEquals(
            listOf(TAKEOFF_COLOUR, RETURN_COLOUR),
            items.map { waypointColour(it.kind, it.commandId) },
        )
    }
}
