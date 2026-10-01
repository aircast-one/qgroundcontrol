package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class TuningChartTest {
    @Test
    fun `history keeps three minutes and the range follows the visible window`() {
        val series = (0..200).fold(emptyList<Sample>()) { list, s -> withSample(list, Sample(s.toDouble(), s.toDouble())) }
        assertEquals(20.0, series.first().seconds, 0.0)
        assertEquals(195.0 to 200.0, visibleRange(listOf(series), 195.0))
        assertNull(visibleRange(listOf(emptyList()), 0.0))
    }
}
