package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CompassMotTest {
    @Test
    fun `a flying aircraft is refused before a busy one, because it is the worse state`() {
        assertEquals(
            "Not while the aircraft is flying.",
            compassMotBlocked(connected = true, busy = true, flying = true),
        )
    }

    @Test
    fun `each refusal names the state it is about`() {
        assertEquals("Connect a vehicle first.", compassMotBlocked(false, false, false))
        assertEquals("Another calibration is running.", compassMotBlocked(true, true, false))
        assertNull(compassMotBlocked(true, false, false))
    }

    @Test
    fun `the steps say to invert the propellers, which is what makes this safe to run`() {
        assertEquals(3, COMPASS_MOT_STEPS.size)
        assertEquals(true, COMPASS_MOT_STEPS[0].contains("turn them over"))
        assertEquals(true, COMPASS_MOT_STEPS[1].contains("cannot move"))
    }
}
