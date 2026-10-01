package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class VideoFitTest {
    private val wide = 16.0 / 9.0

    @Test
    fun `each fit mode sizes the picture like FlightDisplayViewVideo`() {
        assertEquals(1000f to 562.5f, videoContentSize(1000f, 1000f, wide, VIDEO_FIT_WIDTH))
        assertEquals(1777.7778f to 1000f, videoContentSize(1000f, 1000f, wide, VIDEO_FIT_HEIGHT))
        assertEquals(1777.7778f to 1000f, videoContentSize(1000f, 1000f, wide, VIDEO_FILL), "a square box is filled by cropping the sides")
        assertEquals(1000f to 562.5f, videoContentSize(1000f, 1000f, wide, VIDEO_NO_CROP), "no crop letterboxes")
        assertEquals(1000f to 400f, videoContentSize(1000f, 400f, 0.0, VIDEO_NO_CROP), "no aspect known fills the box")
    }

    @Test
    fun `the stream's own size wins over the aspect ratio setting`() {
        assertEquals(4.0 / 3.0, videoAspect(SourceSize(640, 480), wide), 1e-9)
        assertEquals(wide, videoAspect(null, wide), 1e-9)
        assertEquals(0.0, videoAspect(SourceSize(0, 0), null), 1e-9)
    }

    private fun assertEquals(expected: Pair<Float, Float>, actual: Pair<Float, Float>, message: String = "") {
        assertEquals(message, expected.first, actual.first, 0.01f)
        assertEquals(message, expected.second, actual.second, 0.01f)
    }
}
