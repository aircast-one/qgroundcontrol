package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class SetupSummaryTest {
    @Test
    fun `summary rows are keyed by the component they belong to`() {
        val read = setupSummaries(JSONObject("""{"components":[{"name":"Radio","rows":[{"label":"Roll","value":"Channel 1"},{"label":"Pitch","value":"Setup required"}]},{"name":"Frame","rows":[]}]}"""))
        assertEquals(listOf(SummaryLine("Roll", "Channel 1"), SummaryLine("Pitch", "Setup required")), read["Radio"])
        assertEquals(emptyList<SummaryLine>(), read["Frame"])
        assertEquals(emptyMap<String, List<SummaryLine>>(), setupSummaries(null))
    }
}
