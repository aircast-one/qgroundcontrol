package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GpsResilienceIndicatorTest {
    @Test
    fun `marks and sections come from the core`() {
        val shown = gpsResilience(
            JSONObject(
                """{"shown":true,"interference":{"shown":true,"colour":"error"},"authentication":{"shown":false,"colour":"neutral"},""" +
                    """"sections":[{"title":"GPS Resilience Status","rows":[{"label":"GPS Jamming","text":"Not jammed"}]}]}""",
            ),
        )!!
        assertTrue(shown.interference.shown)
        assertEquals("error", shown.interference.colour)
        assertEquals("GPS Jamming" to "Not jammed", shown.sections[0].rows[0])
        assertNull(gpsResilience(JSONObject("""{"shown":false}""")))
    }
}
