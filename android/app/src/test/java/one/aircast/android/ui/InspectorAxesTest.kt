package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test
import java.util.TimeZone

class InspectorAxesTest {
    @Test
    fun `the time axis ticks every third of the window in local mm_ss`() {
        val utc = TimeZone.getTimeZone("UTC")
        val ticks = timeTicks(30_000, 125_000, utc)
        assertEquals(listOf(0f, 1f / 3, 2f / 3, 1f), ticks.map { it.first })
        assertEquals(listOf("01:35", "01:45", "01:55", "02:05"), ticks.map { it.second })
        assertEquals("32:05", timeTicks(30_000, 125_000, TimeZone.getTimeZone("Asia/Kolkata")).last().second)
    }

    @Test
    fun `value ticks climb from the bottom of the range to the top`() {
        val ticks = valueTicks(0.0, 100.0)
        assertEquals("0.000" to 1f, ticks.first().second to ticks.first().first)
        assertEquals("100.0" to 0f, ticks.last().second to ticks.last().first)
    }
}
