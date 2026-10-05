package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class NoVideoPanelTest {
    private val video = VideoReading(available = true, decoding = false, sourceSize = null, summary = "Waiting for a stream.", activeSource = 0, multipleSources = false, cameras = emptyList(), noVideoText = "No video on UDP port 5600")

    @Test
    fun `the detail names the source and how long it has failed`() {
        assertEquals("No video on UDP port 5600 for 12 s", noVideoDetail(video, 12))
        assertEquals("Receiving data — waiting for video for 2 min", noVideoDetail(video.copy(streaming = true), 125))
        assertEquals("1 h 5 min", elapsedText(3900))
    }

    @Test
    fun `a stalled stream says why instead of how long`() {
        val reason = "No answer from 192.168.1.50:8889. Check the address and that this device is on the drone's network."
        assertEquals(reason, noVideoDetail(video.copy(noVideoReason = reason), 70))
    }

    @Test
    fun `the panel reads the active camera's own status like FlightDisplayViewVideo`() {
        val failing = video.copy(cameras = listOf(VideoCamera(slot = 0, title = "Front", status = "Connection failed, retrying", connecting = false, recording = false, configured = true)))
        assertEquals("Connection failed, retrying", activeCameraStatus(failing))
        assertEquals(null, activeCameraStatus(video))
    }

    private fun unavailable(sourceChosen: Boolean, configured: Boolean) = VideoReading(
        available = false, decoding = false, sourceSize = null, summary = "No stream URL is set.", activeSource = 0, multipleSources = false,
        cameras = listOf(VideoCamera(0, "Camera 1", "", connecting = false, recording = false, configured = configured)),
        sourceChosen = sourceChosen,
    )

    @Test
    fun anUnavailableStreamSaysWhatIsMissingAndWhereToFixIt() {
        org.junit.Assert.assertEquals(NoVideoAction.SetUp, unavailableVideoState(unavailable(sourceChosen = false, configured = false))?.action)
        org.junit.Assert.assertEquals("No stream address", unavailableVideoState(unavailable(sourceChosen = true, configured = false))?.title)
        org.junit.Assert.assertEquals(null, unavailableVideoState(unavailable(sourceChosen = true, configured = true).copy(available = true)))
        org.junit.Assert.assertEquals("Enter the stream address below.", unavailableVideoState(unavailable(sourceChosen = true, configured = false), linksToSettings = false)?.detail)
    }

    @Test
    fun `the message avoids the instruments by taking the largest free side`() {
        val free = androidx.compose.ui.geometry.Rect(0f, 100f, 1000f, 1500f)
        val besideTop = androidx.compose.ui.geometry.Rect(0f, 100f, 500f, 220f)
        assertEquals(androidx.compose.ui.geometry.Rect(0f, 230f, 1000f, 1500f), noVideoRegion(free, besideTop, 10f))
        val tallLeft = androidx.compose.ui.geometry.Rect(0f, 100f, 300f, 1500f)
        assertEquals(androidx.compose.ui.geometry.Rect(310f, 100f, 1000f, 1500f), noVideoRegion(free, tallLeft, 10f))
        val movedBelow = androidx.compose.ui.geometry.Rect(300f, 1600f, 700f, 1700f)
        assertEquals(free, noVideoRegion(free, movedBelow, 10f))
        assertEquals(free, noVideoRegion(free, androidx.compose.ui.geometry.Rect.Zero, 10f))
    }
}
