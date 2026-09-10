package one.aircast.mapspike

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
            "4.82 km · 19:37",
            missionSummaryText(view("Distance" to "4.82 km", "Time" to "19:37")),
        )
    }

    @Test
    fun `imperial units come through untouched, because the core already converted them`() {
        assertEquals(
            "2.99 miles · 19:37",
            missionSummaryText(view("Distance" to "2.99 miles", "Time" to "19:37")),
        )
    }

    @Test
    fun `a row the core left out is left out here`() {
        assertEquals("4.82 km", missionSummaryText(view("Distance" to "4.82 km")))
        assertEquals("19:37", missionSummaryText(view("Time" to "19:37")))
    }

    @Test
    fun `rows the summary does not show are ignored`() {
        assertEquals(
            "4.82 km",
            missionSummaryText(view("Batteries" to "2", "Distance" to "4.82 km", "Hover" to "1 km")),
        )
    }

    @Test
    fun `no plan is an empty summary, not a stray separator`() {
        assertEquals("", missionSummaryText(JSONObject("""{"rows":[]}""")))
        assertEquals("", missionSummaryText(null))
    }
}
