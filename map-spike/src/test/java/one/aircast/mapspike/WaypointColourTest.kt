package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class WaypointColourTest {
    @Test
    fun `the commands that change what a plan means each get their own colour`() {
        assertEquals(TAKEOFF_COLOUR, waypointColour("Takeoff"))
        assertEquals(LAND_COLOUR, waypointColour("Land"))
        assertEquals(RETURN_COLOUR, waypointColour("Return To Launch"))
        assertEquals(LOITER_COLOUR, waypointColour("Loiter Time"))
    }

    @Test
    fun `the launch point is not drawn as somewhere the aircraft flies to`() {
        assertEquals(START_COLOUR, waypointColour("Mission Start"))
    }

    @Test
    fun `an ordinary waypoint keeps the plain colour`() {
        assertEquals(WAYPOINT_COLOUR, waypointColour("Waypoint"))
        assertEquals(WAYPOINT_COLOUR, waypointColour(""))
    }

    @Test
    fun `matching ignores case so naming differences do not change the plan`() {
        assertEquals(TAKEOFF_COLOUR, waypointColour("VTOL TAKEOFF"))
        assertEquals(LAND_COLOUR, waypointColour("vtol land"))
    }

    @Test
    fun `every item carries a colour so the layer never reads a missing property`() {
        val items = missionItems(
            org.json.JSONObject(
                """{"kind":"object","items":[{"flownLeg":true,""" +
                    """"coordinate":{"latitude":41.0,"longitude":44.0},"name":"Takeoff"}]}""",
            ),
        )

        val feature = missionFeatures(items).features()!!.single()

        assertEquals(TAKEOFF_COLOUR, feature.getStringProperty(WAYPOINT_COLOUR_PROPERTY))
    }
}
