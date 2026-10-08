package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class VirtualJoystickTest {
    @Test
    fun `the pad maps its position the way JoystickThumbPad does`() {
        assertEquals(StickAxes(0.0, 0.0), stickAxes(0.5f, 0.5f, positiveOnly = false))
        assertEquals(StickAxes(-1.0, 1.0), stickAxes(0f, 0f, positiveOnly = false))
        assertEquals(StickAxes(1.0, 0.5), stickAxes(1f, 0.5f, positiveOnly = true))
        assertEquals(StickAxes(0.0, 0.0), stickAxes(0.5f, 1f, positiveOnly = true))
        assertEquals(1f, restingY(reCenter = false))
        assertEquals(0.5f, restingY(reCenter = true))
    }

    @Test
    fun `the right stick flies roll and pitch unless the layout is left handed`() {
        val throttle = StickAxes(0.1, 0.9)
        val attitude = StickAxes(0.3, -0.4)
        assertEquals(listOf(0.3, -0.4, 0.1, 0.9), joystickValues(throttle, attitude, leftHanded = false))
        assertEquals(listOf(0.1, 0.9, 0.3, -0.4), joystickValues(throttle, attitude, leftHanded = true))
    }

    @Test
    fun `with the sticks never touched the resting positions are sent, as VirtualJoystick sends from the start`() {
        val state = VirtualJoystickState(show = true, sending = true, autoCenterThrottle = false, leftHandedMode = false, leftPositiveOnly = true, rightPositiveOnly = false, periodMs = 40)
        org.junit.Assert.assertEquals(joystickValues(stickAxes(0.5f, 1f, true), stickAxes(0.5f, 0.5f, false), false), stickValues(state, null, null))
        val held = androidx.compose.ui.geometry.Offset(0.2f, 0.3f)
        org.junit.Assert.assertEquals("values follow the current layout, not the one in force when the stick was held", stickValues(state.copy(leftHandedMode = true), held, null), joystickValues(stickAxes(0.2f, 0.3f, true), stickAxes(0.5f, 0.5f, false), true))
        org.junit.Assert.assertEquals("a released throttle stays where it was without auto-centre", androidx.compose.ui.geometry.Offset(0.5f, 0.3f), released(held, false))
    }

    @org.junit.Test
    fun `held sticks are dropped when the joystick hides or throttle auto-centre flips, and kept otherwise`() {
        val shown = VirtualJoystickState(show = true, sending = true, autoCenterThrottle = false, leftHandedMode = false, leftPositiveOnly = true, rightPositiveOnly = false, periodMs = 40)
        org.junit.Assert.assertTrue(sticksReset(shown, shown.copy(show = false)))
        org.junit.Assert.assertTrue(sticksReset(shown, shown.copy(autoCenterThrottle = true)))
        org.junit.Assert.assertTrue(sticksReset(null, null))
        org.junit.Assert.assertFalse(sticksReset(shown, shown.copy(sending = false)))
        org.junit.Assert.assertFalse(sticksReset(shown, shown.copy(leftHandedMode = true)))
    }
}
