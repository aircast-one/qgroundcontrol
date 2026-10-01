package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CenteredThrottleTest {
    @Test
    fun `the radio view carries centered throttle and joystick mode`() {
        val read = radioView(JSONObject("""{"class":"Radio","connected":true,"centeredThrottle":true,"joystickMode":false}"""))!!
        assertTrue(read.centeredThrottle)
        assertFalse(read.joystickMode)
    }
}
