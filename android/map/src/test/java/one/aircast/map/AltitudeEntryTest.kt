package one.aircast.map

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AltitudeEntryTest {
    @Test
    fun `a plain number is an altitude`() {
        assertEquals(47.0, parsedAltitude("47")!!, 1e-9)
        assertEquals(47.5, parsedAltitude(" 47.5 ")!!, 1e-9)
        assertEquals(0.0, parsedAltitude("0")!!, 1e-9)
    }

    @Test
    fun `nonsense and half typed values are refused rather than sent`() {
        assertNull(parsedAltitude(""))
        assertNull(parsedAltitude("."))
        assertNull(parsedAltitude("4x"))
    }

    @Test
    fun `below the launch point and a comma decimal are altitudes, as SimpleMissionItem sets no minimum`() {
        assertEquals(-5.0, parsedAltitude("-5")!!, 1e-9)
        assertEquals(45.5, parsedAltitude("45,5")!!, 1e-9)
        assertNull("both separators is ambiguous", parsedAltitude("1,000.5"))
        assertNull(parsedAltitude("1,000,000"))
    }

    @Test
    fun `the field keeps the decimals QGC shows, so Done on an untouched field writes the same height back`() {
        assertEquals("75", altitudeFieldText(75.0, WAYPOINT_ALTITUDE_DECIMALS))
        assertEquals("45.5", altitudeFieldText(45.5, WAYPOINT_ALTITUDE_DECIMALS))
        assertEquals("164.04", altitudeFieldText(164.0420, RALLY_ALTITUDE_DECIMALS))
        assertEquals("", altitudeFieldText(Double.NaN, WAYPOINT_ALTITUDE_DECIMALS))
        assertEquals("0", altitudeFieldText(-0.04, WAYPOINT_ALTITUDE_DECIMALS))
    }

    @Test
    fun `a survey keeps CameraCalc's 0_1 m floor above the surface`() {
        assertNull(parsedSurfaceDistance("0", 1.0))
        assertEquals(0.1, parsedSurfaceDistance("0,1", 1.0)!!, 1e-9)
        assertNull("0.2 ft is under the 0.1 m floor", parsedSurfaceDistance("0.2", metresPerUnit("ft")))
    }

    @Test
    fun `a rally coordinate is typed within the globe, like RallyPoint's textFieldFacts`() {
        assertEquals(listOf(47.39, null, -180.0, null), listOf(parsedCoordinate("47.39", LATITUDE_LIMIT), parsedCoordinate("91", LATITUDE_LIMIT), parsedCoordinate("-180", LONGITUDE_LIMIT), parsedCoordinate("x", LONGITUDE_LIMIT)))
        assertEquals(-33.87, parsedCoordinate("-33,87", LATITUDE_LIMIT)!!, 1e-9)
    }
}
