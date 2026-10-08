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

    @Test
    fun `the map buttons drop below the video thumbnail only when the thumbnail is drawn`() {
        assertTrue(portraitVideoThumbnail(split = false, reading(available = true, streamEnabled = true, sourceChosen = true)))
        assertFalse("video off draws no thumbnail in the map view", portraitVideoThumbnail(split = false, reading(available = false, streamEnabled = false, sourceChosen = true)))
        assertFalse("the split view has the picture above, not a thumbnail", portraitVideoThumbnail(split = true, reading(available = true, streamEnabled = true, sourceChosen = true)))
        assertFalse(portraitVideoThumbnail(split = false, null))
    }
}
