package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CameraControlTest {

    private fun camera(json: String) = cameraReading(JSONObject(json))

    @Test
    fun `a photo camera offers a photo shutter`() {
        val shutter = shutterFor(
            camera("""{"present":true,"hasModes":true,"modeText":"Photo","mode":0,
                "modeKnown":true,"canPhoto":true,"canRecord":true,
                "isRecording":false,"isTakingPhoto":false}""")!!,
        )!!

        assertEquals("Take Photo", shutter.label)
        assertTrue(shutter.enabled)
        assertFalse(shutter.recording)
    }

    @Test
    fun `a photo already in progress disables the shutter rather than queueing another`() {
        val shutter = shutterFor(
            camera("""{"present":true,"hasModes":true,"modeText":"Photo","mode":0,
                "modeKnown":true,"canPhoto":true,"canRecord":true,
                "isRecording":false,"isTakingPhoto":true}""")!!,
        )!!

        assertFalse(shutter.enabled)
    }

    @Test
    fun `video mode records and then stops`() {
        val idle = shutterFor(
            camera("""{"present":true,"hasModes":true,"modeText":"Video","mode":1,
                "modeKnown":true,"canPhoto":true,"canRecord":true,
                "isRecording":false,"isTakingPhoto":false}""")!!,
        )!!
        val running = shutterFor(
            camera("""{"present":true,"hasModes":true,"modeText":"Video","mode":1,
                "modeKnown":true,"canPhoto":true,"canRecord":true,
                "isRecording":true,"isTakingPhoto":false}""")!!,
        )!!

        assertEquals("Record", idle.label)
        assertEquals("Stop", running.label)
        assertTrue(running.recording)
    }

    @Test
    fun `a camera in a mode it cannot do offers no shutter`() {
        assertNull(
            shutterFor(
                camera("""{"present":true,"hasModes":true,"modeText":"Video","mode":1,
                    "modeKnown":true,"canPhoto":true,"canRecord":false,
                    "isRecording":false,"isTakingPhoto":false}""")!!,
            ),
        )
    }

    @Test
    fun `an unknown mode is not treated as photo`() {
        val unknown = camera("""{"present":true,"hasModes":true,"modeText":"Not set","mode":1,
            "modeKnown":false,"canPhoto":true,"canRecord":true,
            "isRecording":false,"isTakingPhoto":false}""")!!

        assertFalse(unknown.isVideoMode)
    }

    @Test
    fun `no camera present is no controls`() {
        assertNull(cameraReading(null))
        assertNull(cameraReading(JSONObject("""{"present":false}""")))
    }
}
