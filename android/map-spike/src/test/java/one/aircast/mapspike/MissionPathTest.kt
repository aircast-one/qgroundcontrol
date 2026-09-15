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
            2, 3, 41.9, 44.9, "ROI", false, Double.NaN, null, routed = false,
        )

        assertEquals(
            listOf(44.1, 44.2),
            longitudes(listOf(home, first, roi, second), link = false),
        )
    }

    @Test
    fun `an item still being set up is not linked into the route`() {
        val halfMade = MissionItem(
            2, 3, 41.5, 44.15, "Survey", false, Double.NaN, null, routed = false,
        )

        assertEquals(
            listOf(44.1, 44.2),
            longitudes(listOf(home, first, halfMade, second), link = false),
        )
    }

    @Test
    fun `the core decides whether the line starts at home, and this head no longer guesses`() {
        val takeoffButRefused = JSONObject(
            """{"kind":"object","linksStartToHome":false,"items":[{},{"kind":"takeoff"},{}]}""",
        )

        assertFalse(
            "the old rule here was items[1].kind == takeoff, which would say true for this. QGC " +
                "also suppresses the link when an RTL came earlier, and missionitems.rs walks for " +
                "that - a head re-deriving from the item list cannot see it",
            linksStartToHome(takeoffButRefused),
        )
    }

    @Test
    fun `a rover links to home with no takeoff item anywhere`() {
        val rover = JSONObject("""{"kind":"object","linksStartToHome":true,"items":[{},{},{}]}""")

        assertTrue(
            "QGC starts the rule at _controllerVehicle->rover(), the OFFLINE editing vehicle, and " +
                "planningFor never served rover - this head could not have answered it at all",
            linksStartToHome(rover),
        )
    }

    @Test
    fun `an empty plan says false rather than leaving the key out`() {
        assertFalse(linksStartToHome(JSONObject("""{"kind":"object","linksStartToHome":false,"items":[]}""")))
        assertFalse(linksStartToHome(JSONObject("""{"kind":"object"}""")))
        assertFalse(linksStartToHome(null))
    }
}
