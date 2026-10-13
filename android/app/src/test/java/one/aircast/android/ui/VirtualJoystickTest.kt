package one.aircast.android.ui

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.unit.dp
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class VirtualJoystickTest {
    private val copter = VirtualJoystickState(enabled = true, show = true, sending = true, autoCenterThrottle = true, leftHandedMode = false, leftPositiveOnly = true, rightPositiveOnly = false, periodMs = 40)

    private fun fly(state: String, connected: Boolean = true) = flyState(JSONObject().put("class", "FlyState").put("connected", connected).put("state", state))

    private fun thrust(state: VirtualJoystickState, sticks: Sticks) = stickValues(state, sticks)[3]

    @Test
    fun `the pad maps its position the way JoystickThumbPad does`() {
        assertEquals(StickAxes(0.0, 0.0), stickAxes(0.5f, 0.5f, positiveOnly = false))
        assertEquals(StickAxes(-1.0, 1.0), stickAxes(0f, 0f, positiveOnly = false))
        assertEquals(StickAxes(1.0, 0.5), stickAxes(1f, 0.5f, positiveOnly = true))
        assertEquals(StickAxes(0.0, 0.0), stickAxes(0.5f, 1f, positiveOnly = true))
    }

    @Test
    fun `the right stick flies roll and pitch unless the layout is left handed`() {
        val throttle = StickAxes(0.1, 0.9)
        val attitude = StickAxes(0.3, -0.4)
        assertEquals(listOf(0.3, -0.4, 0.1, 0.9), joystickValues(throttle, attitude, leftHanded = false))
        assertEquals(listOf(0.1, 0.9, 0.3, -0.4), joystickValues(throttle, attitude, leftHanded = true))
    }

    @Test
    fun `untouched sticks rest centred, so in the air the throttle holds whatever the centring setting, and values follow the current layout`() {
        val state = copter.copy(autoCenterThrottle = false)
        assertEquals(joystickValues(stickAxes(0.5f, 0.5f, true), stickAxes(0.5f, 0.5f, false), false), stickValues(state, shownSticks(state, null, null, grounded = false, leftHeld = false, rightHeld = false)))
        val held = Offset(0.2f, 0.3f)
        val leftHanded = state.copy(leftHandedMode = true)
        assertEquals(joystickValues(stickAxes(0.2f, 0.3f, true), stickAxes(0.5f, 0.5f, false), true), stickValues(leftHanded, shownSticks(leftHanded, held, CENTRED, grounded = false, leftHeld = true, rightHeld = false)))
    }

    @Test
    fun `on the ground the throttle rests at zero so ArduCopter will arm, and once flying it centres again`() {
        val centred = shownSticks(copter, null, null, grounded = true, leftHeld = false, rightHeld = false)
        assertEquals("ArduPilot refuses to arm with the throttle off zero: Throttle (RC3) is not neutral", 0.0, thrust(copter, centred), 0.0)
        assertEquals("the knob is drawn where the throttle is", Offset(0.5f, 1f), centred.left)
        assertEquals("a held stick sends what the thumb holds", 0.8, thrust(copter, shownSticks(copter, Offset(0.5f, 0.2f), null, grounded = true, leftHeld = true, rightHeld = false)), 0.001)
        assertEquals("in the air an untouched auto-centred throttle holds altitude", 0.5, thrust(copter, shownSticks(copter, null, null, grounded = false, leftHeld = false, rightHeld = false)), 0.0)
        val rover = copter.copy(leftPositiveOnly = false)
        assertEquals("a rover's centred throttle already means stop, so it is not parked at full reverse", 0.0, thrust(rover, shownSticks(rover, null, null, grounded = true, leftHeld = false, rightHeld = false)), 0.0)
    }

    @Test
    fun `only a vehicle known to be on the ground parks the throttle`() {
        assertTrue(onTheGround(fly("disarmed")))
        assertTrue(onTheGround(fly("armed")))
        assertFalse(onTheGround(fly("flying")))
        assertFalse(onTheGround(fly("landing")))
        assertFalse("a lost link may be in the air: zero throttle there would drop it", onTheGround(fly("contactLost")))
        assertFalse(onTheGround(fly("disarmed", connected = false)))
        assertFalse(onTheGround(null))
    }

    @Test
    fun `the throttle stick follows the auto-centre setting and the attitude stick always centres, whichever hand flies it`() {
        val leftHanded = copter.copy(autoCenterThrottle = false, leftHandedMode = true, leftPositiveOnly = false, rightPositiveOnly = true)
        val untouched = shownSticks(leftHanded, null, null, grounded = false, leftHeld = false, rightHeld = false)
        assertEquals("pitch and roll rest centred, not at full pitch", CENTRED, untouched.left)
        assertEquals(CENTRED, untouched.right)
        assertEquals(listOf(0.0, 0.0, 0.0, 0.5), stickValues(leftHanded, untouched))
        assertEquals("on the ground the throttle on the right hand parks at zero", listOf(0.0, 0.0, 0.0, 0.0), stickValues(leftHanded, shownSticks(leftHanded, null, null, grounded = true, leftHeld = false, rightHeld = false)))
        assertEquals(CENTRED, released(Offset(0.9f, 0.1f), throttle = false, autoCenter = false, parked = false))
        assertEquals(CENTRED, released(Offset(0.9f, 0.1f), throttle = true, autoCenter = true, parked = true))
        assertEquals("without auto-centre the throttle stays where it was let go", Offset(0.5f, 0.3f), released(Offset(0.2f, 0.3f), throttle = true, autoCenter = false, parked = false))
        assertEquals("let go on the ground it is kept centred, so it holds rather than drops once the aircraft is flying", CENTRED, released(Offset(0.2f, 0.3f), throttle = true, autoCenter = false, parked = true))
    }

    @Test
    fun `held sticks are dropped when the joystick is turned off or its layout changes, and kept otherwise`() {
        val shown = copter.copy(autoCenterThrottle = false)
        assertTrue(sticksReset(shown, shown.copy(enabled = false)))
        assertFalse("sticks hidden while the autopilot flies keep their released positions", sticksReset(shown, shown.copy(show = false)))
        assertTrue(sticksReset(shown, shown.copy(autoCenterThrottle = true)))
        assertTrue("a throttle held low must not become a full pitch on the other hand", sticksReset(shown, shown.copy(leftHandedMode = true)))
        assertTrue(sticksReset(null, null))
        assertFalse(sticksReset(shown, shown.copy(sending = false)))
    }

    @Test
    fun `a pad is DJI sized, never too small to steer or so big it crowds the screen`() {
        assertEquals(172.6f, stickPad(411.dp).value, 0.1f)
        assertEquals(96.dp, stickPad(200.dp))
        assertEquals(200.dp, stickPad(800.dp))
        assertEquals("a pad shrinks to the room the layout has for it", 150.dp, stickPad(411.dp, 150.dp))
    }


    @Test
    fun `the knob follows the thumb one to one and stays inside the ring`() {
        val travel = knobTravel(100f)
        assertEquals(28f, travel, 0.001f)
        assertEquals(CENTRED, dragged(CENTRED, Offset.Zero, travel))
        assertEquals("a thumb moved the full travel deflects the stick fully", Offset(1f, 0.5f), dragged(CENTRED, Offset(travel, 0f), travel))
        assertEquals(Offset(0f, 1f), dragged(CENTRED, Offset(-500f, 500f), travel))
        assertEquals(Offset(travel, 0f), knobOffset(Offset(1f, 0.5f), travel))
        val corner = knobOffset(Offset(1f, 0f), travel)
        assertEquals("a corner deflection draws on the ring, not outside it", travel, corner.getDistance(), 0.001f)
    }
}
