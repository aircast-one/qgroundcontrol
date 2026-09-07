package one.aircast.mapspike

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WaypointSelectionTest {
    private fun items() = missionItems(
        org.json.JSONObject(
            """{"kind":"object","elements":[""" +
                """{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0}},""" +
                """{"specifiesCoordinate":true,"coordinate":{"latitude":41.1,"longitude":44.1}}]}""",
        ),
    )

    @Test
    fun `only the selected waypoint is marked`() {
        val features = missionFeatures(items(), selectedIndex = 1).features()!!

        assertFalse(features[0].getBooleanProperty(WAYPOINT_SELECTED_PROPERTY))
        assertTrue(features[1].getBooleanProperty(WAYPOINT_SELECTED_PROPERTY))
    }

    @Test
    fun `nothing is marked when nothing is selected`() {
        val features = missionFeatures(items(), selectedIndex = null).features()!!

        assertTrue(features.none { it.getBooleanProperty(WAYPOINT_SELECTED_PROPERTY) })
    }

    @Test
    fun `every feature carries the flag so the layer never reads a missing property`() {
        val features = missionFeatures(items(), selectedIndex = 99).features()!!

        assertTrue(features.all { it.hasProperty(WAYPOINT_SELECTED_PROPERTY) })
    }
}
