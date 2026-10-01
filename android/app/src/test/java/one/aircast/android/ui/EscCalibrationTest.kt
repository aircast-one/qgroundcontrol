package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class EscCalibrationTest {
    @Test
    fun `the dialog shows the core's highlighted prefix, text and config warnings`() {
        val state = escCalibration(JSONObject("""{"open":true,"highlight":"ESC Calibration failed. ","text":"timeout","warnings":["ESC 3 not responding"]}"""))!!
        assertEquals(EscCalibrationState("ESC Calibration failed. ", "timeout", listOf("ESC 3 not responding")), state)
        assertNull(escCalibration(JSONObject("""{"open":false}""")))
    }
}
