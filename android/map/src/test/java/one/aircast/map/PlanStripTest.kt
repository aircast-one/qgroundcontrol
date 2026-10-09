package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PlanStripTest {
    private fun item(index: Int, altitude: Double = Double.NaN) =
        MissionItem(
            index, index, 41.0, 44.0, "Waypoint", true, altitude,
            altitudeText = if (altitude.isNaN()) "" else "${altitude.toInt()} m",
        )

    private val summary = JSONObject("""{"rows":[{"label":"Distance","value":"1.2 km"},{"label":"Time","value":"00:04:30"},{"label":"Furthest from launch","value":"640 m"}]}""")

    @Test
    fun `the strip reads items, the core's distance and time, and the highest altitude`() {
        val items = listOf(item(0, 488.0), item(1, 30.0), item(2, 80.0), item(3, 50.0))

        assertEquals(
            "the planned home sits at ground height above sea level, so counting it would report " +
                "the launch site's elevation as the plan's ceiling",
            listOf(PlanStat("Items", "3"), PlanStat("Distance", "1.2 km"), PlanStat("Time", "04:30"), PlanStat("Max alt", "80 m")),
            planStats(3, items, summary),
        )
    }

    @Test
    fun `a flight under an hour drops the empty hours so the time fits the strip`() {
        val long = JSONObject("""{"rows":[{"label":"Time","value":"01:04:30"}]}""")

        assertEquals(PlanStat("Time", "01:04:30"), planStats(0, emptyList(), long)[1])
    }

    @Test
    fun `a stat the core has not worked out is left off rather than shown as zero`() {
        assertEquals(listOf(PlanStat("Items", "1")), planStats(1, listOf(item(1)), null))
        assertNull(highestAltitude(listOf(item(0, 488.0))))
    }

    @Test
    fun `a selection that is not a mission item is named for what it is`() {
        val items = listOf(MissionItem(2, 2, 41.0, 44.0, "Survey", true))

        assertEquals(
            listOf("Fence corner", "Circular fence", "Rally point 3", "Breach return point", "Survey corner"),
            listOf(MapHit.FenceVertex(0, 1), MapHit.CircleRadius(0), MapHit.Rally(2), MapHit.BreachReturn, MapHit.SurveyVertex(2, 0)).map { selectionTitle(it, items) },
        )
    }

    @Test
    fun `a map tap closes an open editor first, and adds the next point once it is folded`() {
        assertEquals(
            listOf(true, false, false, true),
            listOf(
                tapCloses(MapHit.Waypoint(2), panelOpen = true),
                tapCloses(MapHit.Waypoint(2), panelOpen = false),
                tapCloses(null, panelOpen = true),
                tapCloses(MapHit.Rally(0), panelOpen = false),
            ),
        )
    }

    @Test
    fun `a terrain conflict is counted in legs when the core names them`() {
        assertEquals(listOf("1 leg hits the terrain", "3 legs hit the terrain", "2 items hit the terrain", null), listOf(
            terrainWarning(1, 1), terrainWarning(3, 2), terrainWarning(0, 2), terrainWarning(0, 0),
        ))
    }

    @Test
    fun `advanced names the mission items behind a waypoint with actions`() {
        val folded = MissionItem(3, 3, 41.0, 44.0, "Waypoint", true, 50.0, foldedCommands = 1)
        assertEquals("Mission items 3\u20134", advancedDetail(folded))
        assertNull(advancedDetail(MissionItem(3, 3, 41.0, 44.0, "Waypoint", true, 50.0)))
    }

    @Test
    fun `a takeoff on the planned home hides home, so the map and the strip both read T`() {
        val home = MissionItem(0, 0, 41.0, 44.0, "Home", false, Double.NaN)
        val takeoff = MissionItem(1, 1, 41.0, 44.0, "Takeoff", false, 50.0, abbreviation = "Takeoff")
        val away = takeoff.copy(latitude = 41.001)
        assertEquals(listOf(true, false), listOf(homeCovered(listOf(home, takeoff)), homeCovered(listOf(home, away))))
        assertEquals(listOf("H", "T", "2"), listOf(itemSeal(home), itemSeal(takeoff), itemSeal(MissionItem(2, 2, 41.0, 44.0, "Waypoint", false, 50.0))))
        assertEquals(listOf("Takeoff", "Waypoint 2"), listOf(itemTitle(takeoff), itemTitle(MissionItem(2, 2, 41.0, 44.0, "Waypoint", false, 50.0))))
    }

    @Test
    fun `a template's pattern is the newest one, a rail pattern the one just inserted`() {
        val surveys = listOf(2, 5).map { Survey(it, emptyList(), emptyList(), 0, KIND_SURVEY, "", "surveyAreaPolygon") }
        assertEquals(listOf(5, 2, null), listOf(placedPattern(surveys, NEWEST_PATTERN)?.index, placedPattern(surveys, 2)?.index, placedPattern(surveys, 7)?.index))
    }

    @Test
    fun `with nothing selected the core weighs new items at the end, where they are added`() {
        val items = listOf(0, 1, 2).map { MissionItem(it, it, 41.0, 44.0, "Waypoint", false, 50.0) } + MissionItem(3, 5, 41.0, 44.0, "Survey", false, 50.0)
        assertEquals(5, appendSequence(items))
        assertEquals(0, appendSequence(emptyList()))
    }
}
