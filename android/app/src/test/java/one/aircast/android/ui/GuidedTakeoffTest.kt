package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GuidedTakeoffTest {

    @Test
    fun `the served takeoff carries its range, its sentence and the metric target`() {
        val takeoff = guidedTakeoff(
            JSONObject("""{"available":true,"label":"Takeoff altitude","unit":"m",
                "initial":10.0,"minimum":3.0,"maximum":121.0,"minimumMeters":3.0,
                "target":10.0,"targetMeters":10.0,
                "sentence":"The aircraft will climb to 10.0 m and hold."}"""),
        )!!

        assertEquals(10.0, takeoff.initial!!, 1e-6)
        assertEquals(10.0, takeoff.targetMeters, 1e-6)
        assertEquals("The aircraft will climb to 10.0 m and hold.", takeoff.sentence)
        assertTrue(takeoffRangeUsable(takeoff))
    }

    @Test
    fun `a vehicle with no range yet offers no slider`() {
        val takeoff = guidedTakeoff(
            JSONObject("""{"available":true,"label":"Takeoff altitude","unit":"m",
                "initial":null,"minimum":null,"maximum":null,"minimumMeters":null}"""),
        )!!

        assertNull(takeoff.initial)
        assertFalse(takeoffRangeUsable(takeoff))
    }

    @Test
    fun `an unavailable takeoff is no dialog`() {
        assertNull(guidedTakeoff(null))
        assertNull(guidedTakeoff(JSONObject("""{"available":false}""")))
        assertFalse(takeoffRangeUsable(null))
    }

    @Test
    fun `the argument path carries the target and no separators`() {
        assertEquals("view.guidedTakeoff(10.00)", guidedTakeoffPath(10.0))
        assertFalse(guidedTakeoffPath(10.0).contains(","))
    }

    @Test
    fun `holding Takeoff climbs to the default height and refuses a vehicle with no range`() {
        fun takeoff(initial: Double?, minimum: Double?, maximum: Double?) = GuidedTakeoff("Takeoff altitude", "m", initial, minimum, maximum, "", 10.0)
        assertEquals(10.0, holdTakeoffHeight(takeoff(10.0, 3.0, 121.0))!!, 0.0)
        assertNull(holdTakeoffHeight(takeoff(10.0, null, 121.0)))
        assertNull(holdTakeoffHeight(takeoff(10.0, 5.0, 5.0)))
        assertNull(holdTakeoffHeight(null))
    }
}
