package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class BatteriesTextTest {
    private fun summary(vararg rows: Pair<String, String>) = JSONObject(
        """{"rows":[${rows.joinToString(",") { (l, v) -> """{"label":"$l","value":"$v"}""" }}]}""",
    )

    @Test
    fun `a mission needing more than one battery says so`() {
        assertEquals("3 batteries", batteriesText(summary("Batteries" to "3")))
    }

    @Test
    fun `a mission on a single battery says nothing, because that is the ordinary case`() {
        assertNull(batteriesText(summary("Batteries" to "1")))
    }

    @Test
    fun `no battery row means the core had nothing to say and neither do we`() {
        assertNull(batteriesText(summary("Distance" to "1.15 km")))
        assertNull(batteriesText(null))
    }

    @Test
    fun `the battery count joins the summary after distance and time`() {
        assertEquals(
            "1.15 km · 4:06 · 2 batteries",
            missionSummaryText(summary("Distance" to "1.15 km", "Time" to "4:06", "Batteries" to "2")),
        )
    }

    @Test
    fun `a single-battery mission reads exactly as it did before`() {
        assertEquals(
            "1.15 km · 4:06",
            missionSummaryText(summary("Distance" to "1.15 km", "Time" to "4:06", "Batteries" to "1")),
        )
    }
}
