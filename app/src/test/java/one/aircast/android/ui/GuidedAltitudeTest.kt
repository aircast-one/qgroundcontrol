package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GuidedAltitudeTest {

    @Test
    fun `the served reading carries the sentence and the metric delta`() {
        val reading = guidedAltitude(
            JSONObject("""{"available":true,"label":"Alt (Rel)","unit":"m","current":25.0,
                "currentMeters":25.0,"minimum":0.0,"maximum":121.0,"target":68.5,
                "targetMeters":68.5,"delta":43.5,"deltaMeters":43.5,"sends":true,
                "sentence":"The aircraft will climb 43.5 m to 68.5 m."}"""),
        )!!

        assertEquals("The aircraft will climb 43.5 m to 68.5 m.", reading.sentence)
        assertEquals(43.5, reading.deltaMeters, 1e-6)
        assertTrue(reading.sends)
    }

    @Test
    fun `a vehicle with no altitude yet reads as absent, not as zero`() {
        val reading = guidedAltitude(
            JSONObject("""{"available":true,"label":"Alt (Rel)","unit":"m","current":null,
                "currentMeters":null,"minimum":null,"maximum":null}"""),
        )!!

        assertNull(reading.current)
        assertNull(reading.minimum)
        assertNull(reading.maximum)
        assertFalse(altitudeRangeUsable(reading))
    }

    @Test
    fun `an unavailable view offers nothing`() {
        assertNull(guidedAltitude(null))
        assertNull(guidedAltitude(JSONObject("""{"available":false}""")))
        assertFalse(altitudeRangeUsable(null))
    }

    @Test
    fun `the argument path carries the target and no stray separators`() {
        assertEquals("view.guidedAltitude(68.50)", guidedAltitudePath(68.5))
        assertFalse(guidedAltitudePath(68.5).contains(","))
    }

    @Test
    fun `a pause asks the core for the pause intent`() {
        assertEquals("view.guidedAltitude(25.00,pause)", guidedAltitudePath(25.0, pause = true))
    }
}

class RangeLabelTest {

    @org.junit.Test
    fun `a slider says what its ends are`() {
        org.junit.Assert.assertEquals("3.0 to 121.9 m", rangeLabel(3.0, 121.92, "m"))
        org.junit.Assert.assertEquals("0.1 to 5.0 m/s", rangeLabel(0.1, 5.0, "m/s"))
    }

    @org.junit.Test
    fun `a range with no unit still reads`() {
        org.junit.Assert.assertEquals("1.0 to 8.0", rangeLabel(1.0, 8.0, ""))
    }

    @org.junit.Test
    fun `a range that is not a range says nothing`() {
        org.junit.Assert.assertNull(rangeLabel(null, 5.0, "m"))
        org.junit.Assert.assertNull(rangeLabel(1.0, null, "m"))
        org.junit.Assert.assertNull(rangeLabel(5.0, 5.0, "m"))
        org.junit.Assert.assertNull(rangeLabel(9.0, 5.0, "m"))
    }
}
