package one.aircast.android.ui

import one.aircast.android.hatBits
import one.aircast.android.scaled
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class JoystickScreenTest {
    @Test
    fun the_ax12_built_in_sticks_show_only_where_they_exist_and_say_what_holds_them() {
        assertNull("a phone has no built-in sticks to offer", builtInSticks(JSONObject("""{"available":false,"status":"off"}""")))
        val busy = builtInSticks(JSONObject("""{"available":true,"status":"busy","reason":"RadioMaster's radio app is using the sticks.","owner":"com.Flyshark.RadioMasterAX","receiving":false}"""))!!
        assertEquals(BuiltInSticks("busy", "RadioMaster's radio app is using the sticks.", "com.Flyshark.RadioMasterAX", false), busy)
        assertEquals("RadioMaster's radio app is using the sticks.", builtInSticksText(busy))
        assertEquals("Waiting for the sticks.", builtInSticksText(busy.copy(status = "live", reason = null)))
        assertEquals(true, builtInSticksText(busy.copy(status = "live", reason = null, receiving = true)).startsWith("The AX12's own sticks"))
        val page = joystickPage(JSONObject("""{"available":true,"names":[],"active":null,"builtInSticks":{"available":true,"status":"off","receiving":false}}"""))!!
        assertEquals("off", page.builtInSticks?.status)
        assertNull(page.standbyText)
        val standing = joystickPage(JSONObject("""{"available":true,"names":["Radio"],"active":"Radio","standby":{"capable":true,"enabled":true,"text":"The radio flies the aircraft. 4G is standing by."}}"""))!!
        assertEquals("The radio flies the aircraft. 4G is standing by.", standing.standbyText)
    }

    @Test
    fun the_page_reads_devices_settings_and_axes() {
        assertNull(joystickPage(JSONObject("""{"available":false}""")))
        val page = joystickPage(JSONObject("""{"available":true,"names":["Pad"],"active":"Pad","vehicle":true,"enabled":false,"calibrated":false,
            "settings":[{"name":"useDeadband","type":"bool","label":"Use deadband","units":"","value":true}],
            "state":{"axes":[{"index":0,"raw":1200,"function":"roll","label":"Roll"}]}}"""))!!
        assertEquals("Pad", page.active)
        assertEquals(true, page.settings.single().value)
        assertEquals(JoystickAxis(0, 1200, "Roll"), page.axes.single())
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
        val step = joystickCalibration(JSONObject("""{"calibrating":true,"status":{"bullets":false,"lines":["Move the Throttle stick"]},"nextText":"Next","nextEnabled":false,"cancelEnabled":true,"oneSidedVisible":true}"""))
        assertEquals(JoystickCalibration(true, StatusLines(false, listOf("Move the Throttle stick")), "Next", false, true, true), step)
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
    fun `calibration reads the step count, the held stick, the live sticks and the check warning`() {
        val cal = joystickCalibration(JSONObject("""{"calibrating":true,"step":4,"steps":10,"holding":0.5,"checking":true,"warning":"Roll moved the opposite way to yaw.",
            "live":{"roll":-0.9,"pitch":0.1,"yaw":0,"throttle":-1},"livePositions":[0,-1,-0.9,0.1]}"""))
        assertEquals(listOf(4, 10, 0.5, true, "Roll moved the opposite way to yaw."), listOf(cal.step, cal.steps, cal.holding, cal.checking, cal.warning))
        assertEquals(mapOf("roll" to -0.9, "pitch" to 0.1, "yaw" to 0.0, "throttle" to -1.0), cal.live)
        assertEquals(listOf(0.0, -1.0, -0.9, 0.1), cal.livePositions)
        val idle = joystickCalibration(null)
        assertEquals(listOf(null, null, null, false), listOf(idle.step, idle.holding, idle.warning, idle.checking))
    }

    @Test
    fun `the stick mode and whether the sticks reach the aircraft are worded by the core`() {
        val page = joystickPage(JSONObject("""{"available":true,"names":["Pad"],"active":"Pad","enabled":true,"transmitterMode":2,"transmitterModeText":"Throttle and yaw on the left stick, pitch and roll on the right",
            "sendingText":"Not sending: no input from the radio. Check its cable and move a stick","state":{"commanding":false,"notCommandingReason":"noSample"}}"""))!!
        assertEquals("Throttle and yaw on the left stick, pitch and roll on the right", page.transmitterModeText)
        assertEquals(listOf(false, "Not sending: no input from the radio. Check its cable and move a stick"), listOf(page.commanding, page.sendingText))
        assertNull(joystickPage(JSONObject("""{"available":true,"names":["Pad"],"active":"Pad","enabled":false,"sendingText":null}"""))!!.sendingText)
    }

    @Test
    fun `a step the core hands as a list is read as one`() {
        assertEquals(StatusLines(true, listOf("Center all sticks.", "Click Next to continue")), statusLines(JSONObject("""{"bullets":true,"lines":["Center all sticks.","Click Next to continue"]}""")))
        assertEquals(StatusLines(false, listOf("Move the Roll stick all the way to the right")), statusLines(JSONObject("""{"bullets":false,"lines":["Move the Roll stick all the way to the right"]}""")))
        assertEquals(StatusLines(false, emptyList()), joystickCalibration(null).status)
    }

    @Test
    fun a_setting_the_core_hides_is_not_listed() {
        val page = joystickPage(JSONObject("""{"available":true,"names":["Pad"],"active":"Pad","settings":[{"name":"negativeThrust","type":"bool","label":"Negative thrust","units":"","value":false,"visible":false},{"name":"exponentialPct","type":"double","label":"Exponential","units":"%","value":0}]}"""))!!
        assertEquals(listOf("exponentialPct"), page.settings.map { it.name })
        val ranged = joystickPage(JSONObject("""{"available":true,"names":["Pad"],"active":"Pad","settings":[{"name":"exponentialPct","type":"double","label":"Stick Exponential","units":"%","value":0,"slider":{"from":0.0,"to":50.0,"decimals":1}}]}"""))!!
        assertEquals(one.aircast.android.bridge.FactSlider(0f, 50f, 1, ""), ranged.settings.single().slider)
    }
}
