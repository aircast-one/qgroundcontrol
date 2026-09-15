package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MotorGateTest {

    private fun frame(
        connected: Boolean = true,
        armed: Boolean = false,
        contactLost: String = "false",
    ) = JSONObject(
        """{"kind":"object","class":"Frame","connected":$connected,"armed":$armed,
           "contactLost":$contactLost,"apmFirmware":false,"motorCount":4}""",
    )

    @Test
    fun `an armed vehicle is not a vehicle to spin a motor on`() {
        assertFalse(canTest(motorGate(frame(armed = true)), propsOff = true))
        assertEquals(
            "The vehicle is armed. Disarm it before testing a motor.",
            motorRefusal(motorGate(frame(armed = true))),
        )
    }

    @Test
    fun `a vehicle that stopped answering is not one to spin a motor on`() {
        assertFalse(canTest(motorGate(frame(contactLost = "true")), propsOff = true))
    }

    @Test
    fun `nobody watching is neither lost nor fine, and must not refuse`() {
        assertTrue(
            "frame.rs withholds contactLost when the watch is off, because the raw flag stays " +
                "false however long the vehicle has been silent. Refusing on unknown would ground " +
                "a motor test that has nothing wrong with it",
            canTest(motorGate(frame(contactLost = "null")), propsOff = true),
        )
        assertNull(motorRefusal(motorGate(frame(contactLost = "null"))))
    }

    @Test
    fun `no vehicle means nothing will answer`() {
        assertFalse(canTest(motorGate(frame(connected = false)), propsOff = true))
        assertFalse(canTest(motorGate(null), propsOff = true))
    }

    @Test
    fun `the safety switch is still required with everything else in order`() {
        assertFalse(canTest(motorGate(frame()), propsOff = false))
        assertTrue(canTest(motorGate(frame()), propsOff = true))
    }

    @Test
    fun `stop is not a test and does not inherit a test's gate`() {
        assertTrue(
            "the two conditions that are REASONS to stop - the vehicle arming while the motors " +
                "turn, or the safety switch going off - both disabled the one control an operator " +
                "reaches for. A control is enabled by what its own action requires, and sending " +
                "zero throttle requires only a vehicle to send it to",
            canStop(motorGate(frame(armed = true))),
        )
        assertTrue(canStop(motorGate(frame(contactLost = "true"))))
        assertFalse(canStop(motorGate(frame(connected = false))))
    }
}
