package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MotorsTest {
    @Test
    fun `a vehicle that names its motor count is believed`() {
        assertEquals(6, motorCount(6))
        assertNull(motorCountNotice(6))
    }

    @Test
    fun `an airframe with no published layout gets eight buttons rather than none`() {
        assertEquals(8, motorCount(null))
    }

    @Test
    fun `an unknown count warns with MotorComponent's text`() {
        assertEquals("Warning: Unable to determine motor count", motorCountNotice(null))
    }

    @Test
    fun `an absent count is a null rather than a number to be sifted`() {
        assertNull(
            "view.frame already drops the -1 QGC answers for an airframe it has no layout for, " +
                "and holds the count back for a submarine until its parameters arrive - a rule " +
                "this head cannot express from a raw motorCount",
            reportedMotors(JSONObject("""{"connected":true,"motorCount":null}""")),
        )
        assertEquals(4, reportedMotors(JSONObject("""{"connected":true,"motorCount":4}""")))
        assertNull(reportedMotors(JSONObject("""{"connected":false}""")))
        assertNull(reportedMotors(null))
    }

    @Test
    fun `a count below one is not a layout`() {
        assertNull(reportedMotors(JSONObject("""{"motorCount":0}""")))
    }
}
