package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class GuidedAltitudeTest {

    @Test
    fun `the range always contains where the aircraft actually is`() {
        val above = altitudeRange(settingMin = 5.0, settingMax = 50.0, current = 80.0)
        val below = altitudeRange(settingMin = 5.0, settingMax = 50.0, current = 1.0)

        assertTrue(above.max >= 80.0)
        assertTrue(below.min <= 1.0)
    }

    @Test
    fun `reversed settings still produce a usable range`() {
        val range = altitudeRange(settingMin = 50.0, settingMax = 5.0, current = 20.0)

        assertTrue(range.min < range.max)
    }

    @Test
    fun `the command sent is a delta, not the target`() {
        assertEquals(15.0, altitudeDelta(target = 25.0, current = 10.0), 0.001)
        assertEquals(-15.0, altitudeDelta(target = 10.0, current = 25.0), 0.001)
    }

    @Test
    fun `the summary names the direction and both altitudes`() {
        assertEquals(
            "The aircraft will climb 15.0 m to 25.0 m.",
            altitudeChangeSummary(target = 25.0, current = 10.0),
        )
        assertEquals(
            "The aircraft will descend 15.0 m to 10.0 m.",
            altitudeChangeSummary(target = 10.0, current = 25.0),
        )
    }

    @Test
    fun `no movement is stated rather than described as a climb`() {
        assertTrue(altitudeChangeSummary(target = 10.0, current = 10.0).contains("will not move"))
    }

    @Test
    fun `the summary and the button agree on what counts as no movement`() {
        val edge = ALTITUDE_DEADBAND_METERS / 2

        assertTrue(altitudeChangeSummary(10.0 + edge, 10.0).contains("will not move"))
        assertTrue(!altitudeChangeIsUseful(10.0 + edge, 10.0))
        assertTrue(altitudeChangeIsUseful(10.0 + ALTITUDE_DEADBAND_METERS * 2, 10.0))
    }
}
