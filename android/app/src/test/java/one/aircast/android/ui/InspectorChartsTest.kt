package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class InspectorChartsTest {
    private val view = JSONObject(
        """{"timeScales":["5 Sec","10 Sec"],"ranges":["Auto","1"],"selectedCharted":[{"field":"roll","chart":0}],
           "charts":[{"rangeX":0,"rangeY":0,"windowMs":5000,"yMin":-1,"yMax":1,"room":true,"plots":[{"label":"ATTITUDE.roll","field":"roll","colour":0,"points":[[2000,0.5]]}]},
                     {"rangeX":1,"rangeY":1,"windowMs":10000,"yMin":null,"yMax":null,"room":false,"plots":[]}]}""",
    )

    @Test
    fun `charts parse with their plots and which fields are charted`() {
        val charts = inspectorCharts(view) ?: error("parsed")
        assertEquals(listOf("5 Sec", "10 Sec"), charts.timeScales)
        assertEquals(listOf(2000L to 0.5), charts.charts[0].plots[0].points)
        assertEquals(mapOf("roll" to 0), charts.charted)
        assertEquals(null, charts.charts[1].yMin)
    }

    @Test
    fun `a field charts once, text never, and a full chart takes no more`() {
        val charts = inspectorCharts(view)
        assertTrue(chartToggleEnabled(charts, "roll", "float", 0))
        assertFalse(chartToggleEnabled(charts, "roll", "float", 1))
        assertFalse(chartToggleEnabled(charts, "text", "char[50]", 0))
        assertTrue(chartToggleEnabled(charts, "pitch", "float", 0))
        assertFalse(chartToggleEnabled(charts, "pitch", "float", 1))
    }

    @Test
    fun `a sample lands by its age and value`() {
        assertEquals(0.6f to 0.25f, chartPoint(2000, 0.5, 5000, -1.0, 1.0))
        assertEquals(1f to 1f, chartPoint(0, -5.0, 5000, -1.0, 1.0))
    }
}
