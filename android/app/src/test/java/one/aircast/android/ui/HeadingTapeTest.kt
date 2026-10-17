package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
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
        assertEquals(160f, tapeX(0f, 320f), 0.001f)
        assertEquals(0f, tapeX(-60f, 320f), 0.001f)
        assertEquals(320f, tapeX(60f, 320f), 0.001f)
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
    fun `ticks cover the visible span in ten degree steps`() {
        val ticks = tapeTicks(102f)
        assertEquals(50, ticks.first())
        assertEquals(160, ticks.last())
        assertEquals(12, ticks.size)
    }

    @Test
    fun `labels clear the readout and the edges`() {
        assertFalse(tapeLabelShown(12f))
        assertTrue(tapeLabelShown(-18f))
        assertTrue(tapeLabelShown(48f))
        assertFalse(tapeLabelShown(58f))
    }

    @Test
    fun `the readout is whole degrees`() {
        assertEquals("7°", headingReadout(7.4f))
        assertEquals("0°", headingReadout(359.6f))
        assertEquals("270°", headingReadout(-90f))
    }
}
