package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class BreachReturnTest {
    @Test
    fun `the breach return point and its altitude read from the fences view`() {
        val read = breachReturn(
            JSONObject(
                """{"breachReturnPoint":{"latitude":47.4,"longitude":8.5},""" +
                    """"breachReturnAltitude":{"value":98.4,"units":"ft","path":"plan.geoFenceController.breachReturnAltitude"}}""",
            ),
        )
        assertEquals(BreachReturn(TrackPoint(47.4, 8.5), 98.4, "ft", "plan.geoFenceController.breachReturnAltitude"), read)
        assertNull(breachReturn(JSONObject("""{"breachReturnPoint":null}""")))
        assertEquals("98.4", breachAltitudeText(98.4))
    }
}
