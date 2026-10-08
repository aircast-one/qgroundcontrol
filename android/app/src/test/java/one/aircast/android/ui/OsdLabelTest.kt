package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class OsdLabelTest {
    @Test
    fun `the flying readings shorten to DJI's letters and anything else keeps its name`() {
        assertEquals(listOf("D", "H", "H.S", "V.S"), listOf("Distance to home", "Alt (Rel)", "Ground Speed", "Climb Rate").map(::osdLabel))
        assertEquals("FLIGHT TIME", osdLabel("Flight time"))
        assertEquals("D.OP", osdLabel("From you"))
        assertEquals(listOf("D", "H", "H.S", "V.S"), listOf("distanceToHome", "altitudeRelative", "groundSpeed", "climbRate").map(::osdLabel))
    }
}

class DjiChromeTest {
    @Test
    fun `the status title splits into plain mode text and a note for the pill`() {
        assertEquals("Manual", osdModeText("Manual · Not fully ready"))
        assertEquals("Not fully ready", osdStatusNote("Manual · Not fully ready"))
        assertEquals(null, osdStatusNote("Manual"))
    }

    @Test
    fun `speeds go on the small top row and the rest on the large one`() {
        assertEquals(listOf(true, true, false, false), listOf("Ground Speed", "Climb Rate", "Alt (Rel)", "Distance to home").map(::osdIsSpeed))
    }

    @Test
    fun `the battery ring reads the percentage out of the cell text`() {
        assertEquals(53, batteryPercent("B1 53%"))
        assertEquals(null, batteryPercent("No battery"))
    }

    @Test
    fun `the mini-map opens as a thumbnail unless the pilot chose otherwise`() {
        assertEquals(MiniMap.Thumb, miniMapNamed(null))
        assertEquals(MiniMap.Compass, miniMapNamed("Compass"))
        assertEquals(MiniMap.Thumb, miniMapNamed("Unknown"))
    }
}

class FlightTimeTest {
    @Test
    fun `flight time reads as DJI's minutes and seconds`() {
        assertEquals("00'00\"", flightTimeText(null))
        assertEquals("00'00\"", flightTimeText(0.0))
        assertEquals("01'15\"", flightTimeText(75.4))
        assertEquals("72'05\"", flightTimeText(4325.0))
    }
}

class FlightTimeReadTest {
    @Test
    fun `the seconds come out of the fact the bridge wraps them in`() {
        assertEquals(10.325, flightTimeSeconds(org.json.JSONObject("""{"kind":"value","value":{"kind":"fact","value":10.325}}""")))
        assertEquals(null, flightTimeSeconds(org.json.JSONObject("""{"kind":"value","value":null}""")))
    }
}

class StreamShutterTest {
    @Test
    fun `the stream shutter is a video shutter that says what a tap will do`() {
        assertEquals(Triple("Start recording", false, true), streamShutter(false, true, null).let { Triple(it.label, it.recording, it.video) })
        assertEquals(Triple("Stop recording", true, true), streamShutter(true, true, 5).let { Triple(it.label, it.recording, it.video) })
    }

    @Test
    fun `a stream that cannot record shows a dead shutter, but one already recording can still stop`() {
        assertEquals(false, streamShutter(false, recordable = false, elapsedSeconds = null).enabled)
        assertEquals(true, streamShutter(true, recordable = false, elapsedSeconds = 3).enabled)
    }

    @Test
    fun `the stream recording clock reads like the camera's`() {
        assertEquals("REC 00:01:05", shutterReadout(streamShutter(true, true, 65)))
        assertEquals(null, shutterReadout(streamShutter(false, true, null)))
        assertEquals("01:00:00", recordClock(3600))
    }
}

class RailLabelTest {
    @Test
    fun `rail labels drop the hold instruction`() {
        assertEquals(listOf("Land", "Return", "Take off"), listOf("Hold to land", "Return", "Hold to take off").map(::railLabel))
    }
}
