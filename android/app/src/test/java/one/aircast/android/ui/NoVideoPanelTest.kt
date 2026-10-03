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
    fun `the panel reads the active camera's own status like FlightDisplayViewVideo`() {
        val failing = video.copy(cameras = listOf(VideoCamera(slot = 0, title = "Front", status = "Connection failed, retrying", connecting = false, recording = false, configured = true)))
        assertEquals("Connection failed, retrying", activeCameraStatus(failing))
        assertEquals(null, activeCameraStatus(video))
    }
}
