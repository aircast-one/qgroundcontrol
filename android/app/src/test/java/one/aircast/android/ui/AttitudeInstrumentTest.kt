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
                """{"available":true,"roll":-12.5,"pitch":4,"heading":7.4,"headingText":"007",""" +
                    """"courseOverGround":null,"headingToHome":190,"headingToNextWaypoint":null,"noseUp":false}""",
            ),
        )!!
        assertEquals(-12.5f, reading.roll)
        assertEquals("007", reading.headingText)
        assertNull(reading.courseOverGround)
        assertEquals(190f, reading.headingToHome)
        assertFalse(reading.noseUp)
    }

    @Test
    fun `no vehicle draws nothing`() {
        assertNull(attitude(JSONObject("""{"available":false}""")))
        assertNull(attitude(null))
    }

    @Test
    fun `pitch moves the horizon by the span QGC uses`() {
        assertEquals(40f, pitchOffset(45f, 20f))
        assertEquals(0f, pitchOffset(0f, 20f))
        assertFalse(0 in ladderAngles())
        assertEquals(36, ladderAngles().size)
    }
}
