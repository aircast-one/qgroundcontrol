package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ActuatorTestSectionTest {
    private val motor = TestChannel("Motor 1", 101, 0.0, 1.0, null, true)
    private val servo = TestChannel("Servo 1", 201, -1.0, 1.0, 0.0, false)

    @Test
    fun `a motor slider has a stop zone below min that snaps like ActuatorSlider`() {
        assertEquals(-0.15, motor.from, 1e-9)
        assertEquals(-0.15, motor.rest, 1e-9)
        assertEquals(-0.15, snapped(motor, -0.1), 1e-9)
        assertEquals(0.0, snapped(motor, -0.05), 1e-9)
        assertNull(sentValue(motor, -0.15))
        assertEquals(0.4, sentValue(motor, 0.4)!!, 1e-9)
    }

    @Test
    fun `a servo slider starts at its default and has no stop zone`() {
        assertEquals(-1.0, servo.from, 1e-9)
        assertEquals(0.0, servo.rest, 1e-9)
        assertEquals(-1.0, snapped(servo, -1.0), 1e-9)
    }
}

class ActuatorActionsTest {
    @Test
    fun `action groups read from the outputs view`() {
        val groups = actuatorActions(
            org.json.JSONObject("""{"actions":[{"label":"Set Spin Direction 1","type":4,"actions":[{"label":"Motor 1","function":101}]}]}"""),
        )
        assertEquals(listOf(ActuatorActionGroup("Set Spin Direction 1", 4, listOf(ActuatorActionChoice("Motor 1", 101)))), groups)
    }
}
