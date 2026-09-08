package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.maplibre.geojson.LineString

class MissionPathTest {
    private fun item(index: Int, longitude: Double) =
        MissionItem(index, index + 1, 41.0, longitude, "Waypoint", false, Double.NaN)

    private val home = item(0, 44.0)
    private val first = item(1, 44.1)
    private val second = item(2, 44.2)

    private fun longitudes(items: List<MissionItem>, link: Boolean): List<Double>? =
        (missionPath(items, link)?.geometry() as? LineString)
            ?.coordinates()?.map { it.longitude() }

    @Test
    fun `without a takeoff the line does not start at the planned home`() {
        assertEquals(listOf(44.1, 44.2), longitudes(listOf(home, first, second), link = false))
    }

    @Test
    fun `with a takeoff the line starts at the planned home`() {
        assertEquals(listOf(44.0, 44.1, 44.2), longitudes(listOf(home, first, second), link = true))
    }

    @Test
    fun `one point left after dropping home is no line at all`() {
        assertNull(missionPath(listOf(home, first), linkStartToHome = false))
    }

    @Test
    fun `an item with an exit is flown through rather than touched`() {
        val survey = MissionItem(
            1, 2, 41.0, 44.1, "Survey", false, Double.NaN, TrackPoint(41.0, 44.15),
        )

        assertEquals(
            listOf(44.1, 44.15, 44.2),
            longitudes(listOf(home, survey, second), link = false),
        )
    }

    @Test
    fun `an exit equal to the entry is not a second point`() {
        assertEquals(listOf(44.1, 44.2), longitudes(listOf(home, first, second), link = false))
    }

    @Test
    fun `the route does not detour through a standalone coordinate`() {
        val roi = MissionItem(
            2, 3, 41.9, 44.9, "ROI", false, Double.NaN, null, standalone = true,
        )

        assertEquals(
            listOf(44.1, 44.2),
            longitudes(listOf(home, first, roi, second), link = false),
        )
    }

    @Test
    fun `the rule is the same one the distance uses`() {
        val takeoffFirst = JSONObject(
            """{"kind":"object","elements":[{},{"isTakeoffItem":true},{}]}""",
        )
        val waypointFirst = JSONObject("""{"kind":"object","elements":[{},{},{}]}""")

        assertTrue(linksStartToHome(takeoffFirst))
        assertFalse(linksStartToHome(waypointFirst))
        assertFalse(linksStartToHome(null))
    }
}
