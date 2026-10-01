package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ApmServosScreenTest {
    @Test
    fun an_idle_output_has_no_reading_and_the_steppers_move_by_one() {
        val servos = apmServos(
            JSONObject(
                """{"servos":[{"index":3,"pwm":null,"position":null,
                "min":{"kind":"control","control":"number","name":"SERVO3_MIN","path":"p.min","value":1100,"valueString":"1100"},
                "function":null,"trim":null,"max":null,"reversed":null}]}""",
            ),
        )
        assertEquals(3, servos.single().index)
        assertNull(servos.single().pwm)
        assertNull(servos.single().position)
        assertEquals(1099.0, stepped(servos.single().min!!, -1))
        assertEquals(emptyList<ApmServo>(), apmServos(null))
    }
}
