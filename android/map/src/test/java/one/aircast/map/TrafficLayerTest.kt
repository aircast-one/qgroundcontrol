package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class TrafficLayerTest {
    @Test
    fun `contacts become marks labelled as the map item labels them`() {
        val marks = trafficMarks(
            JSONObject(
                """{"units":{"altitude":"ft"},"contacts":[""" +
                    """{"latitude":47.4,"longitude":8.5,"headingDegrees":90,"alert":true,"altitude":3280.8,"callsign":"UAL123"},""" +
                    """{"latitude":null,"longitude":8.5,"alert":false,"altitude":100,"callsign":"X"}]}""",
            ),
        )
        assertEquals(1, marks.size)
        assertEquals("3281 ft\nUAL123", marks[0].label)
        assertTrue(marks[0].alert)
        assertEquals("", trafficLabel(null, "ft", "UAL123"))
    }
}
