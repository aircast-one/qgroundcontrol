package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GimbalTakeControlTest {
    @Test
    fun `another holder of the gimbal asks to take control instead of failing quietly`() {
        gimbalAsksForControl.value = false
        assertNull(gimbalRefusal(JSONObject("""{"ok":false,"refusal":"othersHaveControl","reason":"Command not sent. Another user has control of the gimbal."}""")))
        assertTrue(gimbalAsksForControl.value)
        assertEquals("The gimbal is not ready yet.", gimbalRefusal(JSONObject("""{"ok":false,"refusal":"notReady","reason":"The gimbal is not ready yet."}""")))
        gimbalAsksForControl.value = false
    }

    @Test
    fun `a joystick refusal raised in the core asks once the serial moves, never on the first read`() {
        assertEquals(false, serialAsks(null, 3))
        assertEquals(false, serialAsks(3, 3))
        assertEquals(true, serialAsks(3, 4))
    }

    @Test
    fun `a drag aims by its move over half the view width, as CameraAimArea's sensitivity`() {
        assertEquals(0.5f, aimFraction(100f, 400), 0f)
        assertEquals(-0.25f, aimFraction(-50f, 400), 0f)
    }

    @Test
    fun `each way a drag on the video can steer the gimbal maps to its two settings and back`() {
        assertEquals(VideoGimbalMode.entries, VideoGimbalMode.entries.map { videoGimbalMode(OnScreenGimbal(it.onScreen, it.clickAndDrag)) })
        assertEquals(VideoGimbalMode.Drag, videoGimbalMode(OnScreenGimbal(enabled = false, clickAndDrag = true)))
        assertEquals(listOf("Drag to move", "Tap to point", "Hold and drag"), VideoGimbalMode.entries.map { it.label })
    }

    @Test
    fun `the switch on the video steps through every mode and back to the first`() {
        assertEquals(listOf(VideoGimbalMode.Point, VideoGimbalMode.Hold, VideoGimbalMode.Drag), VideoGimbalMode.entries.map(::nextVideoGimbalMode))
    }
}
