package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CameraResetTest {

    private fun reading(present: Boolean) = cameraReading(
        JSONObject("""{"kind":"object","class":"Camera","present":$present,"title":"ZR30"}"""),
    )

    @Test
    fun `no camera means nothing to reset`() {
        assertFalse(
            "resetSettings is sent to currentCameraInstance; with no camera that path " +
                "resolves to nothing and the button would report a success that never happened",
            cameraCanReset(reading(false)),
        )
        assertFalse(cameraCanReset(null))
    }

    @Test
    fun `a present camera can be reset`() {
        assertTrue(cameraCanReset(reading(true)))
    }

    @Test
    fun `the wording is the one Qt confirms with`() {
        assertEquals("Reset Camera to Factory Settings", RESET_TITLE)
        assertEquals("Confirm resetting all settings?", RESET_PROMPT)
    }

    @Test
    fun `the reset is sent to the open camera, not the manager`() {
        assertTrue(
            "cameraManager has no resetSettings; the invokable is on the camera itself",
            CAMERA_RESET.endsWith("currentCameraInstance.resetSettings"),
        )
    }
}
