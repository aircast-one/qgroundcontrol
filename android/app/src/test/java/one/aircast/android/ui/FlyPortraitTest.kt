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

    @Test
    fun `a swipe past the threshold reads as its main direction, and a short one is ignored`() {
        org.junit.Assert.assertEquals(VideoSwipe.Up, videoSwipe(androidx.compose.ui.geometry.Offset(5f, -80f), 40f))
        org.junit.Assert.assertEquals(VideoSwipe.Down, videoSwipe(androidx.compose.ui.geometry.Offset(-10f, 60f), 40f))
        org.junit.Assert.assertNull("too short", videoSwipe(androidx.compose.ui.geometry.Offset(0f, -30f), 40f))
        org.junit.Assert.assertEquals(listOf(1, -1, null), listOf(VideoSwipe.Left, VideoSwipe.Right, VideoSwipe.Up).map(::cameraStep))
        org.junit.Assert.assertEquals("sideways switches the camera rather than hiding", VideoSwipe.Right, videoSwipe(androidx.compose.ui.geometry.Offset(120f, -60f), 40f))
        org.junit.Assert.assertEquals(VideoSwipe.Left, videoSwipe(androidx.compose.ui.geometry.Offset(-90f, 10f), 40f))
    }

    @Test
    fun `a dragged picture-in-picture settles on the corner nearest where it was dropped`() {
        val geometry = PipGeometry(
            width = 1080f,
            pip = androidx.compose.ui.geometry.Size(440f, 248f),
            inset = 33f,
            pipTop = 300f,
            bottomStartTop = 1600f,
            bottomEndTop = 1550f,
            split = androidx.compose.ui.geometry.Rect(0f, 200f, 1080f, 808f),
            full = androidx.compose.ui.geometry.Rect(0f, 0f, 1080f, 2200f),
        )
        val at = { x: Float, y: Float -> geometry.nearest(androidx.compose.ui.geometry.Offset(x, y)) }
        org.junit.Assert.assertEquals(listOf(PipCorner.TopStart, PipCorner.TopEnd, PipCorner.BottomStart, PipCorner.BottomEnd), listOf(at(300f, 400f), at(800f, 400f), at(300f, 1700f), at(800f, 1650f)))
        org.junit.Assert.assertEquals(androidx.compose.ui.geometry.Offset(607f, 300f), geometry.anchor(PipCorner.TopEnd))
        org.junit.Assert.assertEquals("a bottom corner sits above the deck, clear of the compass or the scale bar", androidx.compose.ui.geometry.Offset(33f, 1600f), geometry.anchor(PipCorner.BottomStart))
        val finger = androidx.compose.ui.geometry.Offset(540f, 500f)
        org.junit.Assert.assertEquals("pulled out of the split view, the picture is centred under the finger", finger, geometry.pip(PipCorner.TopEnd, geometry.dragToCentre(PipCorner.TopEnd, finger)).center)
    }

    @Test
    fun `a picture hides toward the edge it sits on and grows away from it`() {
        org.junit.Assert.assertEquals(VideoSwipe.Up to VideoSwipe.Down, hidingSwipe(PipCorner.TopEnd) to growingSwipe(PipCorner.TopEnd))
        org.junit.Assert.assertEquals(VideoSwipe.Down to VideoSwipe.Up, hidingSwipe(PipCorner.BottomStart) to growingSwipe(PipCorner.BottomStart))
    }

    @Test
    fun `the controls beside the video make room for the picture or for its tab`() {
        org.junit.Assert.assertEquals(0f, pipRoom(thumbnail = false, tucked = false).value)
        assertTrue(pipRoom(thumbnail = true, tucked = true) < pipRoom(thumbnail = true, tucked = false))
    }
}
