package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class TuningChartTest {
    @Test
    fun `history keeps three minutes`() {
        val series = (0..200).fold(emptyList<Sample>()) { list, s -> withSample(list, Sample(s.toDouble(), s.toDouble())) }
        assertEquals(20.0, series.first().seconds, 0.0)
    }

    @Test
    fun `the y range only grows, snapped outward to fives, as PIDTuning's adjustYAxis does`() {
        assertNull(grownRange(null, emptyList()))
        assertEquals("the first point sets both ends", 3.0 to 3.0, grownRange(null, listOf(3.0)))
        assertEquals(-10.0 to 10.0, grownRange(3.0 to 3.0, listOf(-1.0, 7.0)))
        assertEquals("a value inside the range never shrinks it", -10.0 to 10.0, grownRange(-10.0 to 10.0, listOf(2.0)))
    }
}
