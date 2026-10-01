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

    @Test
    fun `the surveyed position is saved as the fixed base only once the survey is valid`() {
        val surveying = RtkStatus(active = true, valid = false, satellites = 9, durationS = 10.0, accuracyM = 3.0, latitude = 47.1, longitude = 8.5, altitudeM = 400.0)
        assertNull(basePositionWrites(surveying))
        assertEquals(
            listOf(
                "settings.rtkSettings.fixedBasePositionLatitude" to 47.1,
                "settings.rtkSettings.fixedBasePositionLongitude" to 8.5,
                "settings.rtkSettings.fixedBasePositionAltitude" to 400.0,
                "settings.rtkSettings.fixedBasePositionAccuracy" to 3.0,
            ),
            basePositionWrites(surveying.copy(valid = true)),
        )
        assertNull(basePositionWrites(surveying.copy(valid = true, latitude = null)))
    }
}
