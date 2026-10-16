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
    fun `the seconds are the value of the fact the core answers with`() {
        assertEquals(10.325, flightTimeSeconds(org.json.JSONObject("""{"kind":"fact","value":10.325}""")))
        assertEquals(null, flightTimeSeconds(org.json.JSONObject("""{"kind":"fact","value":null}""")))
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
        assertEquals(listOf("Land", "Return", "Take off"), listOf("Slide to land", "Return", "Slide to take off").map(::railLabel))
    }
}

class BatteryRingTest {
    @Test
    fun `the ring splits the charge into land, return and free zones like DJI`() {
        assertEquals(
            listOf(RingSegment(0.0, 10.0, RingZone.Reserve), RingSegment(10.0, 30.0, RingZone.Return), RingSegment(30.0, 60.0, RingZone.Free)),
            batteryRingSegments(60.0, 10.0, 30.0, null),
        )
    }

    @Test
    fun `below the return point the ring shows only land and return`() {
        assertEquals(listOf(RingSegment(0.0, 10.0, RingZone.Reserve), RingSegment(10.0, 20.0, RingZone.Return)), batteryRingSegments(20.0, 10.0, 30.0, null))
    }

    @Test
    fun `without a return estimate the charge above the reserve is free`() {
        assertEquals(listOf(RingSegment(0.0, 60.0, RingZone.Free)), batteryRingSegments(60.0, 0.0, null, null))
        assertEquals(listOf(RingSegment(0.0, 5.0, RingZone.Reserve)), batteryRingSegments(5.0, 10.0, null, null))
    }

    @Test
    fun `the yellow usage at the tip grows with the draw and never runs past empty`() {
        assertEquals(RingSegment(42.0, 60.0, RingZone.Usage), batteryRingSegments(60.0, 0.0, null, 18.0).last())
        assertEquals(RingSegment(0.0, 10.0, RingZone.Usage), batteryRingSegments(10.0, 0.0, null, 25.0).last())
    }
}
