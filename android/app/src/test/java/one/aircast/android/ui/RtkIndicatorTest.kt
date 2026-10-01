package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RtkIndicatorTest {
    @Test
    fun `a disconnected base shows nothing`() {
        assertNull(rtkStatus(JSONObject("""{"connected":false}""")))
        assertNull(rtkStatus(null))
    }

    @Test
    fun `a survey in progress reads like QGC's RTK section`() {
        val status = rtkStatus(JSONObject("""{"connected":true,"active":true,"valid":false,"numSatellites":17,"currentDuration":42.0,"currentAccuracy":3.456}"""))!!
        assertEquals("Survey-in Active", rtkHeadline(status))
        assertEquals(listOf("Satellites" to "17", "Duration" to "42 s", "Current Accuracy" to "3.5 m"), rtkRows(status))
    }

    @Test
    fun `a finished survey streams and hides an accuracy it has not reported`() {
        val status = rtkStatus(JSONObject("""{"connected":true,"active":false,"valid":true,"numSatellites":null,"currentDuration":null,"currentAccuracy":null}"""))!!
        assertEquals("RTK Streaming", rtkHeadline(status))
        assertEquals(listOf("Satellites" to "", "Duration" to "0 s"), rtkRows(status))
    }
}
