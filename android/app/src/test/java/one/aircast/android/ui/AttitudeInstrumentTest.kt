package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

class AttitudeInstrumentTest {
    @Test
    fun `reads the angles and hides pointers the core withholds`() {
        val reading = attitude(
            JSONObject(
                """{"available":true,"roll":-12.5,"pitch":4,"heading":7.4,"headingText":"7\u00b0",""" +
                    """"courseOverGround":null,"headingToHome":190,"headingToNextWaypoint":null,"noseUp":false}""",
            ),
        )!!
        assertEquals(-12.5f, reading.roll)
        assertEquals("7\u00b0", reading.headingText)
        assertNull(reading.courseOverGround)
        assertEquals(190f, reading.headingToHome)
        assertFalse(reading.noseUp)
    }

    @Test
    fun `no vehicle reads as nothing, and the panel draws the bare dial at north`() {
        assertNull(attitude(JSONObject("""{"available":false}""")))
        assertNull(attitude(null))
        assertEquals(0f, NO_VEHICLE_ATTITUDE.heading)
        assertEquals("", NO_VEHICLE_ATTITUDE.headingText)
    }

    @Test
    fun `the dial reads home and the pilot whatever the compass setting`() {
        val read = attitude(JSONObject("""{"available":true,"heading":10,"homeBearing":190,"pilotBearing":45.5,"headingToHome":null}"""))!!
        assertEquals(190f, read.homeBearing)
        assertEquals(45.5f, read.pilotBearing)
        assertNull(attitude(JSONObject("""{"available":true,"heading":10,"homeBearing":null,"pilotBearing":null}"""))!!.pilotBearing)
    }

    @Test
    fun `pitch moves the horizon by the span QGC uses`() {
        assertEquals(40f, pitchOffset(45f, 20f))
        assertEquals(0f, pitchOffset(0f, 20f))
    }
}
