package one.aircast.android.ui

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class FlyPortraitTest {
    private fun reading(available: Boolean, streamEnabled: Boolean, sourceChosen: Boolean) = VideoReading(
        available = available,
        decoding = false,
        sourceSize = null,
        summary = "",
        activeSource = 0,
        cameras = emptyList(),
        streamEnabled = streamEnabled,
        sourceChosen = sourceChosen,
    )

    @Test
    fun `the camera pane stays in portrait while video is switched off so it can be turned back on`() {
        assertTrue(portraitShowsCamera(reading(available = true, streamEnabled = true, sourceChosen = true)))
        assertTrue(portraitShowsCamera(reading(available = false, streamEnabled = false, sourceChosen = true)))
        assertFalse(portraitShowsCamera(reading(available = false, streamEnabled = true, sourceChosen = false)))
        assertFalse(portraitShowsCamera(reading(available = false, streamEnabled = false, sourceChosen = false)))
        assertFalse(portraitShowsCamera(null))
    }
}
