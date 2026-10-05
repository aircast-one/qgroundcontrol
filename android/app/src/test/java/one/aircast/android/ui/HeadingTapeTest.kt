package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class HeadingTapeTest {
    @Test
    fun `a bearing is placed relative to the heading across north`() {
        assertEquals(20f, relativeBearing(10f, 350f), 0.001f)
        assertEquals(-20f, relativeBearing(350f, 10f), 0.001f)
        assertEquals(-180f, relativeBearing(180f, 0f), 0.001f)
        assertEquals(0f, relativeBearing(725f, 5f), 0.001f)
    }

    @Test
    fun `the heading sits in the middle and the span fills the width`() {
        assertEquals(120f, tapeX(0f, 240f), 0.001f)
        assertEquals(0f, tapeX(-60f, 240f), 0.001f)
        assertEquals(240f, tapeX(60f, 240f), 0.001f)
    }

    @Test
    fun `cardinal points are letters and the rest are degrees`() {
        assertEquals("N", tapeLabel(360))
        assertEquals("E", tapeLabel(90))
        assertEquals("W", tapeLabel(-90))
        assertEquals("330", tapeLabel(-30))
        assertEquals("120", tapeLabel(120))
    }

    @Test
    fun `ticks cover the visible span in five degree steps`() {
        val ticks = tapeTicks(2f)
        assertEquals(-55, ticks.first())
        assertEquals(60, ticks.last())
        assertEquals(24, ticks.size)
    }

    @Test
    fun `the readout is three digits like an OSD`() {
        assertEquals("007", headingReadout(7.4f))
        assertEquals("000", headingReadout(359.6f))
        assertEquals("270", headingReadout(-90f))
    }
}
