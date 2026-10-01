package one.aircast.android.ui

import one.aircast.android.hatBits
import one.aircast.android.scaled
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class JoystickScreenTest {
    @Test
    fun the_page_reads_devices_settings_and_axes() {
        assertNull(joystickPage(JSONObject("""{"available":false}""")))
        val page = joystickPage(JSONObject("""{"available":true,"names":["Pad"],"active":"Pad","vehicle":true,"enabled":false,"calibrated":false,
            "settings":[{"name":"useDeadband","type":"bool","label":"Use deadband","units":"","value":true}],
            "state":{"axes":[{"index":0,"raw":1200,"function":"roll"}]}}"""))!!
        assertEquals("Pad", page.active)
        assertEquals(true, page.settings.single().value)
        assertEquals(JoystickAxis(0, 1200, "roll"), page.axes.single())
    }

    @Test
    fun gamepad_values_scale_to_the_core_range_and_the_dpad_becomes_hat_bits() {
        assertEquals(32767, scaled(1f))
        assertEquals(-32767, scaled(-2f))
        assertEquals(0x01 or 0x08, hatBits(-1f, -1f))
        assertEquals(0x04 or 0x02, hatBits(1f, 1f))
        assertEquals(0, hatBits(0.2f, -0.2f))
    }

    @Test
    fun the_calibration_panel_reads_the_core_wizard() {
        val idle = joystickCalibration(null)
        assertEquals("Calibrate", idle.nextText)
        val step = joystickCalibration(JSONObject("""{"calibrating":true,"statusText":"Move the Throttle stick","nextText":"Next","nextEnabled":false,"cancelEnabled":true,"oneSidedVisible":true}"""))
        assertEquals(JoystickCalibration(true, "Move the Throttle stick", "Next", false, true, true), step)
    }

    @Test
    fun buttons_show_their_action_and_whether_they_are_held() {
        val page = joystickPage(JSONObject("""{"available":true,"names":["Pad"],"active":"Pad","vehicle":true,"enabled":true,"calibrated":true,"settings":[],
            "state":{"axes":[],"buttons":[{"index":0,"action":null,"repeat":false,"event":"none"},{"index":1,"action":"Step Zoom In","repeat":true,"event":"repeat"}]},
            "assignableActions":[{"action":"No Action","canRepeat":false},{"action":"Step Zoom In","canRepeat":true}]}"""))!!
        assertEquals(listOf(JoystickButton(0, "No Action", false, false), JoystickButton(1, "Step Zoom In", true, true)), page.buttons)
        assertEquals(AssignableAction("Step Zoom In", true), page.actions[1])
    }

    @Test
    fun `calibration reads the stick diagram positions`() {
        val cal = joystickCalibration(org.json.JSONObject("""{"calibrating":true,"stickPositions":[0,1,0,0],"singleStickDisplay":false}"""))
        org.junit.Assert.assertEquals(listOf(0, 1, 0, 0), cal.stickPositions)
        org.junit.Assert.assertEquals(listOf(0, 0, 0, 0), joystickCalibration(null).stickPositions)
    }

    @Test
    fun a_setting_the_core_hides_is_not_listed() {
        val page = joystickPage(JSONObject("""{"available":true,"names":["Pad"],"active":"Pad","settings":[{"name":"negativeThrust","type":"bool","label":"Negative thrust","units":"","value":false,"visible":false},{"name":"exponentialPct","type":"double","label":"Exponential","units":"%","value":0}]}"""))!!
        assertEquals(listOf("exponentialPct"), page.settings.map { it.name })
    }
}
