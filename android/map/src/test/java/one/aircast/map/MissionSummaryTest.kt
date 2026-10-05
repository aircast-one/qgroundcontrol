package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class MissionSummaryTest {
    private fun view(vararg rows: Pair<String, String>) = JSONObject(
        """{"rows":[${rows.joinToString(",") { """{"label":"${it.first}","value":"${it.second}"}""" }}]}""",
    )

    @Test
    fun `the summary reads the core's words rather than formatting its own`() {
        assertEquals(
            "Distance 4.82 km · Time 19:37",
            missionSummaryText(view("Distance" to "4.82 km", "Time" to "19:37")),
        )
    }

    @Test
    fun `imperial units come through untouched, because the core already converted them`() {
        assertEquals(
            "Distance 2.99 miles · Time 19:37",
            missionSummaryText(view("Distance" to "2.99 miles", "Time" to "19:37")),
        )
    }

    @Test
    fun `a row the core left out is left out here`() {
        assertEquals("Distance 4.82 km", missionSummaryText(view("Distance" to "4.82 km")))
        assertEquals("Time 19:37", missionSummaryText(view("Time" to "19:37")))
    }

    @Test
    fun `rows the summary does not show are ignored`() {
        assertEquals(
            "Distance 4.82 km",
            missionSummaryText(view("Cruise" to "3 km", "Distance" to "4.82 km", "Hover" to "1 km")),
        )
    }

    @Test
    fun `no plan is an empty summary, not a stray separator`() {
        assertEquals("", missionSummaryText(JSONObject("""{"rows":[]}""")))
        assertEquals("", missionSummaryText(null))
    }

    @Test
    fun `the furthest point from launch reads as QGC's Max telem, each total labelled like PlanToolBarIndicators`() {
        assertEquals(
            "Distance 4.82 km · Time 19:37 · Max telem 1.2 km",
            missionSummaryText(view("Distance" to "4.82 km", "Time" to "19:37", "Furthest from launch" to "1.2 km")),
        )
    }
}
