package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class JoystickIndicatorTest {
    @Test
    fun `the badge reads the core's indicator and hides without a joystick`() {
        val badge = joystickBadge(JSONObject("""{"indicator":{"heading":"Xbox","enabledText":"No","warn":true,"typeText":"Gamepad","inputsText":"6 axes, 15 buttons"}}"""))
        assertEquals(JoystickBadge("Xbox", "No", true, "Gamepad", "6 axes, 15 buttons"), badge)
        assertNull(joystickBadge(JSONObject("""{"indicator":null}""")))
    }
}
