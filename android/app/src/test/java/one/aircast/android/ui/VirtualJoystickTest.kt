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
}
