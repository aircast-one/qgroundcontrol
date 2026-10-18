package one.aircast.android.ui

import androidx.compose.ui.geometry.Offset
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceFlightsTest {
    private fun marker(offset: Int, label: String) = TraceMarker(offset, label, DeviceLevel.Neutral)

    @Test
    fun `reads the flight log the device serves`() {
        val view = parseFlights(
            JSONObject(
                """{"flights":[
                {"id":"f1","title":"Tue 22 Sep, 14:00","summary":"Lasted 22m10s","badge":{"level":3,"text":"3 events"},
                 "sections":[{"title":"Sortie 1","span":"14:02:10 – 14:12:40 · 10m30s",
                   "videos":[{"camera":"cam2","label":"Video · cam2","url":"/api/recordings/cam2/video?from=a&to=b","share":{"label":"Share version · cam2","confirm":"Send about 1.2 MB?","camera":"cam2","from":"a","to":"b"}}],
                   "lines":[{"index":1,"clock":"14:02:14","text":"cam stopped sending video","level":2,"alert":true,"repeats":"repeated ×4 until 14:02:44"}]}],
                 "trace":{"step":30,"startUnix":1790085600,"tzOffset":10800,
                   "lanes":[{"key":"signal","label":"Modem signal","chip":"Signal","values":[80,41],"notes":["4G","3G"],"unit":"percent","max":100,"default":true,"peak":"41%"}],
                   "markers":[{"offset":60,"label":"14:02:05 modem","level":2}],
                   "spans":[{"label":"Sortie 1","from":0.1,"to":0.9}]},
                 "csv":"/api/flights/f1/samples.csv","json":"/api/flights/f1"},
                {"id":"f2","title":"Wed","badge":{"level":0,"text":"in progress"},"open":true,"sections":[],"dropped":"12 more events happened"}]}""",
            ),
        )
        val traced = view?.flights?.get(0)
        val live = view?.flights?.get(1)
        assertEquals(FlightLine(1, "14:02:14", "cam stopped sending video", DeviceLevel.Blocked, alert = true, repeats = "repeated ×4 until 14:02:44", heldBack = false), traced?.sections?.first()?.lines?.first())
        assertEquals("cam2", traced?.sections?.first()?.videos?.first()?.share?.camera)
        assertEquals(TraceLane("signal", "Modem signal", "Signal", listOf(80.0, 41.0), listOf("4G", "3G"), "percent", 100.0, default = true, peak = "41%"), traced?.trace?.lanes?.first())
        assertEquals(DeviceLevel.Blocked, traced?.trace?.markers?.first()?.level)
        assertEquals(TraceSpan("Sortie 1", 0.1, 0.9), traced?.trace?.spans?.first())
        assertTrue(traced != null && hasLines(traced))
        assertNull(live?.trace)
        assertTrue(live?.open == true)
        assertFalse(live != null && hasLines(live))
        assertNull(parseFlights(JSONObject("{}")))
        assertEquals(emptyList<FlightEntry>(), parseFlights(JSONObject("""{"flights":[]}"""))?.flights)
    }

    @Test
    fun `a copied link opens the fly app straight on that flight`() {
        assertEquals("http://falcon.local/fly/#device=%2Fflights%3Fflight%3D20260922-140000", flightLink("http://falcon.local/fly/", "20260922-140000"))
    }

    @Test
    fun `a flight opens with its own defaults until someone chooses`() {
        assertEquals(listOf("signal", "bitrate"), laneKeys(null, listOf("signal", "bitrate")))
        assertEquals(listOf("cpu"), laneKeys(listOf("cpu"), listOf("signal", "bitrate")))
    }

    @Test
    fun `choosing nothing stays chosen`() {
        assertEquals(emptyList<String>(), laneKeys(emptyList(), listOf("signal")))
    }

    @Test
    fun `a chip adds the lane it names and takes it away again`() {
        assertEquals(listOf("signal", "cpu"), toggled(listOf("signal"), "cpu"))
        assertEquals(listOf("cpu"), toggled(listOf("signal", "cpu"), "signal"))
    }

    @Test
    fun `the line breaks where the value is unknown`() {
        assertEquals(listOf(listOf(Offset(0f, 1f), Offset(25f, 10f)), listOf(Offset(75f, 19f), Offset(100f, 1f))), segments(listOf(100.0, 50.0, -1.0, 0.0, 100.0), 100.0))
    }

    @Test
    fun `without a ceiling the line scales to its own peak`() {
        assertEquals(listOf(listOf(Offset(0f, 19f), Offset(50f, 10f), Offset(100f, 1f))), segments(listOf(0.0, 250000.0, 500000.0), 0.0))
    }

    @Test
    fun `a single point draws nothing`() {
        assertEquals(emptyList<List<Offset>>(), segments(listOf(42.0), 100.0))
        assertEquals(emptyList<List<Offset>>(), segments(emptyList(), 100.0))
    }

    @Test
    fun `a lane gets evenly spaced clock ticks from first to last sample`() {
        val ticked = ticks(1_700_000_000, 30, 13) { it.toString() }
        assertEquals(listOf(0.0, 25.0, 50.0, 75.0, 100.0), ticked.map { it.x })
        assertEquals("1700000000", ticked.first().label)
        assertEquals("1700000360", ticked[4].label)
        assertEquals(emptyList<LaneTick>(), ticks(1, 30, 1) { it.toString() })
    }

    @Test
    fun `markers land where their moment falls on the series, or not at all`() {
        assertEquals(
            listOf(MarkerLine(0.0, "a", DeviceLevel.Neutral, 0), MarkerLine(50.0, "b", DeviceLevel.Neutral, 1)),
            markerLines(listOf(marker(0, "a"), marker(180, "b"), marker(361, "late"), marker(-1, "early")), 30, 13),
        )
    }

    @Test
    fun `a zoomed lane keeps each marker on its moment and its index on its line`() {
        assertEquals(listOf(MarkerLine(50.0, "inside", DeviceLevel.Neutral, 1)), markerLines(listOf(marker(30, "before"), marker(90, "inside")), 30, 3, 60))
    }

    @Test
    fun `the caption names the span in minutes or hours`() {
        assertEquals("6m", spanLabel(360))
        assertEquals("1h 15m", spanLabel(4500))
    }

    @Test
    fun `the axis reads in the device's clock, not the phone's`() {
        assertEquals("22:13", clockAt(1_700_000_000, 0, false))
        assertEquals("01:13:20", clockAt(1_700_000_000, 3 * 3600, true))
    }

    @Test
    fun `gridlines sit at quarters of the ceiling with the unit's label`() {
        assertEquals(listOf("25%", "50%", "75%"), gridLines(100.0, "percent").map { it.label })
        assertEquals(listOf("500 kbps", "1.0 Mbps", "1.5 Mbps"), gridLines(250000.0, "rate").map { it.label })
        assertEquals(emptyList<LaneGridLine>(), gridLines(0.0, "percent"))
    }

    @Test
    fun `a tap reads the nearest sample with its note`() {
        assertEquals(LaneReadout(2, 100.0, "18% · 3G"), readoutAt(listOf(80.0, 41.0, 18.0), listOf("4G (LTE)", "3G", "3G"), 0.9, "percent"))
        assertEquals("no reading", readoutAt(listOf(80.0, -1.0), emptyList(), 1.0, "percent")?.text)
        assertNull(readoutAt(emptyList(), emptyList(), 0.5, "rate"))
    }

    @Test
    fun `counts read as bare numbers`() {
        assertEquals("512", formatValue(512.4, "count"))
    }

    @Test
    fun `distances read in metres until a kilometre`() {
        assertEquals("640 m", formatValue(640.0, "meters"))
        assertEquals("1.3 km", formatValue(1250.0, "meters"))
        assertEquals(listOf("500 m", "1.0 km", "1.5 km"), gridLines(2000.0, "meters").map { it.label })
    }

    @Test
    fun `a zoom window picks a run of samples and never fewer than two`() {
        assertEquals(0..13, zoomSlice(14, null))
        assertEquals(3..7, zoomSlice(14, Zoom(0.25, 0.5)))
        assertEquals(12..13, zoomSlice(14, Zoom(0.99, 1.0)))
        assertEquals(4..5, zoomSlice(14, Zoom(0.3, 0.31)))
    }

    @Test
    fun `a drag inside a zoom narrows the zoom it is in`() {
        assertEquals(Zoom(0.2, 0.6), composeZoom(null, 0.6, 0.2))
        assertEquals(Zoom(0.5, 0.75), composeZoom(Zoom(0.5, 1.0), 0.0, 0.5))
    }

    @Test
    fun `each lane's unit is spelled the way that lane is read`() {
        assertEquals("0.7", formatValue(0.72, "number"))
        assertEquals("2.0 GB", formatValue(2048.0, "mb"))
        assertEquals("512 MB", formatValue(512.0, "mb"))
        assertEquals("45 s", formatValue(45.0, "seconds"))
        assertEquals("3 m", formatValue(180.0, "seconds"))
        assertEquals("73%", formatValue(73.0, "percent"))
        assertEquals("4", formatValue(4.0, "count"))
    }

    @Test
    fun `a render reports how far it got and why it stopped`() {
        val uploading = parseRenderJob(JSONObject("""{"id":"r1","state":"uploading","sent":300,"total":1200}"""))
        assertTrue(renderRunning(uploading))
        assertEquals(25, renderPercent(uploading))
        assertEquals(0, renderPercent(parseRenderJob(JSONObject("""{"state":"uploading"}"""))))
        assertFalse(renderRunning(parseRenderJob(JSONObject("""{"state":"done","url":"https://r2/x.mp4"}"""))))
        assertEquals("The cloud could not render this video.", parseRenderJob(JSONObject("""{"state":"failed"}""")).error)
        assertEquals("the cloud could not render this video", parseRenderJob(JSONObject("""{"state":"failed","error":"the cloud could not render this video"}""")).error)
    }
}
