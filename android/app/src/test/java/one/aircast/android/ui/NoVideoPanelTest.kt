package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class NoVideoPanelTest {
    private val video = VideoReading(available = true, decoding = false, sourceSize = null, summary = "Waiting for a stream.", activeSource = 0, cameras = emptyList(), noVideoText = "No video on UDP port 5600")

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
        val failing = video.copy(cameras = listOf(VideoCamera(slot = 0, status = "Connection failed, retrying", configured = true)))
        assertEquals("Connection failed, retrying", activeCameraStatus(failing))
        assertEquals(null, activeCameraStatus(video))
    }

    private fun unavailable(sourceChosen: Boolean, configured: Boolean, streamEnabled: Boolean = true) = VideoReading(
        available = false, decoding = false, sourceSize = null, summary = "No stream URL is set.", activeSource = 0,
        cameras = listOf(VideoCamera(0, "", configured = configured)),
        sourceChosen = sourceChosen,
        streamEnabled = streamEnabled,
    )

    @Test
    fun anUnavailableStreamSaysWhatIsMissingAndWhereToFixIt() {
        assertEquals(NoVideoAction.SetUp, unavailableVideoState(unavailable(sourceChosen = false, configured = false))?.action)
        assertEquals("No stream address", unavailableVideoState(unavailable(sourceChosen = true, configured = false))?.title)
        assertEquals(NoVideoAction.Settings, unavailableVideoState(unavailable(sourceChosen = true, configured = true))?.action)
        assertEquals(null, unavailableVideoState(unavailable(sourceChosen = true, configured = true).copy(available = true)))
    }

    @Test
    fun `a stream switched off offers to turn it on rather than sending the pilot to settings`() {
        val off = unavailableVideoState(unavailable(sourceChosen = true, configured = true, streamEnabled = false))
        assertEquals("Video off", off?.title)
        assertEquals(NoVideoAction.TurnOn, off?.action)
        assertEquals(NoVideoAction.TurnOn, unavailableVideoState(unavailable(sourceChosen = false, configured = false, streamEnabled = false))?.action)
    }

    @Test
    fun `while armed only the one-tap turn-on survives, never a trip to settings`() {
        val off = whileArmed(unavailableVideoState(unavailable(sourceChosen = true, configured = true, streamEnabled = false))!!)
        assertEquals(NoVideoAction.TurnOn, off.action)
        assertEquals("", off.detail)
        assertEquals(NoVideoAction.None, whileArmed(unavailableVideoState(unavailable(sourceChosen = true, configured = false))!!).action)
        assertEquals(NoVideoAction.None, whileArmed(unavailableVideoState(unavailable(sourceChosen = false, configured = false))!!).action)
    }

    @Test
    fun `the missing-video copy points at the Video sources page on the Transmission tab`() {
        assertEquals("Add a camera in Settings \u203a Transmission \u203a Video sources.", unavailableVideoState(unavailable(sourceChosen = false, configured = false))?.detail)
        assertEquals("Enter the stream address in Settings \u203a Transmission \u203a Video sources.", unavailableVideoState(unavailable(sourceChosen = true, configured = false))?.detail)
    }

    @Test
    fun `the video button is named for the Video sources page it opens`() {
        val navigation = AppNavigationState()
        val button = videoSourcesButton(navigation)
        button.onClick()
        assertEquals("Video sources", button.label)
        assertEquals(VIDEO_SOURCES_PAGE, navigation.settingsPage)
    }

    private fun rect(left: Float, top: Float, right: Float, bottom: Float) = androidx.compose.ui.geometry.Rect(left, top, right, bottom)

    @Test
    fun `the message takes the largest free side of the instruments, or the whole area when they are elsewhere`() {
        val free = rect(0f, 100f, 1000f, 1500f)
        assertEquals(rect(0f, 230f, 1000f, 1500f), messageRegion(free, listOf(rect(0f, 100f, 500f, 220f)), 10f, 200f, 50f))
        assertEquals(rect(310f, 100f, 1000f, 1500f), messageRegion(free, listOf(rect(0f, 100f, 300f, 1500f)), 10f, 200f, 50f))
        assertEquals(free, messageRegion(free, listOf(rect(300f, 1600f, 700f, 1700f)), 10f, 200f, 50f))
        assertEquals(free, messageRegion(free, emptyList(), 10f, 200f, 50f))
    }

    @Test
    fun `the message clears every widget, looking past the first side it finds`() {
        val free = rect(0f, 100f, 1000f, 600f)
        val tape = rect(0f, 100f, 400f, 200f)
        val fleet = rect(0f, 210f, 900f, 600f)
        assertEquals(rect(410f, 100f, 1000f, 200f), messageRegion(free, listOf(tape, fleet), 10f, 200f, 50f))
    }

    @Test
    fun `with no gap big enough for the pill there is no message rather than a squeezed one`() {
        val free = rect(0f, 100f, 1000f, 600f)
        assertNull(messageRegion(free, listOf(rect(0f, 100f, 950f, 600f)), 10f, 200f, 50f))
        assertNull(messageRegion(rect(0f, 0f, 150f, 600f), emptyList(), 10f, 200f, 50f))
    }
}

class CentredRegionTest {
    @org.junit.Test
    fun `a message beside the mini-map still centres on the screen when it fits`() {
        val room = androidx.compose.ui.geometry.Rect(410f, 100f, 1990f, 900f)
        org.junit.Assert.assertEquals(androidx.compose.ui.geometry.Rect(410f, 100f, 1590f, 900f), centredRegion(room, 1000f, 400f))
    }

    @org.junit.Test
    fun `a room too lopsided to centre in keeps its own shape`() {
        val room = androidx.compose.ui.geometry.Rect(900f, 100f, 1990f, 900f)
        org.junit.Assert.assertEquals(room, centredRegion(room, 1000f, 400f))
    }
}
