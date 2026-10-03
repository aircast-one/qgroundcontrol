package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class EscCalibrationTest {
    @Test
    fun `the dialog shows the core's highlighted prefix and text`() {
        val state = escCalibration(JSONObject("""{"open":true,"highlight":"ESC Calibration failed. ","text":"timeout","running":false}"""))!!
        assertEquals(EscCalibrationState("ESC Calibration failed. ", "timeout", running = false), state)
        assertNull(escCalibration(JSONObject("""{"open":false}""")))
    }
}
