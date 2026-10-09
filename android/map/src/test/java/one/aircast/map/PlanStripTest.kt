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
}
