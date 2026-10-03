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

    @org.junit.Test
    fun `a summary reads as one line of its values, as Penpot's setup rows`() {
        val frame = listOf(SummaryLine("Frame Class", "Quad"), SummaryLine("Frame Type", "X"), SummaryLine("Firmware Version", "Unknown"))
        org.junit.Assert.assertEquals("Quad \u00b7 X", summaryGlance(frame))
        org.junit.Assert.assertEquals("RTL \u00b7 Land", summaryGlance(listOf(SummaryLine("a", "RTL"), SummaryLine("b", "Land"), SummaryLine("c", "RTL"))))
        org.junit.Assert.assertNull(summaryGlance(emptyList()))
        org.junit.Assert.assertEquals("Analog voltage and current \u00b7 5000 mAh", summaryGlance(listOf(SummaryLine("Battery monitor", "Analog Voltage and Current"), SummaryLine("Capacity", "5000 mAh"))))
        val radio = listOf(SummaryLine("Roll", "Channel 1"), SummaryLine("Pitch", "Channel 2"), SummaryLine("Throttle", "Channel 3"))
        org.junit.Assert.assertEquals("Roll 1 \u00b7 Pitch 2 \u00b7 Throttle 3", summaryGlance(radio))
        org.junit.Assert.assertEquals("Ready \u00b7 Not supported (over APM 4.1)", summaryGlance(listOf(SummaryLine("Compass", "Ready"), SummaryLine("Airspeed", "Not Supported(Over APM 4.1)"))))
    }
}
