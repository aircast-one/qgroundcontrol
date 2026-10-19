package one.aircast.android.ui

import androidx.compose.ui.geometry.Offset
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceFlightsTest {
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
        assertEquals(30, traced?.trace?.optInt("step"))
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
    fun `draws the chart the core computed in the core's plot units`() {
        val chart = parseFlightChart(
            JSONObject(
                """{"lanes":[{"key":"signal","label":"Modem signal","chip":"Signal","peak":"41%","default":true,
                  "lines":[[[0,5],[25,50.5]],[[75,95],[100,5]]],
                  "grid":[{"y":72.5,"label":"25%"}],
                  "markers":[{"x":50,"label":"14:02:05 modem","index":1}],
                  "readings":["14:00:00 · 80% · 4G","14:00:30 · no reading"],
                  "ticks":[{"x":0,"label":"14:00"}],
                  "span":"6m of flight"}],
                "spans":[{"label":"Sortie 1","from":0.1,"to":0.9}]}""",
            ),
        )
        val lane = chart?.lanes?.first()
        assertEquals(listOf(listOf(Offset(0f, 5f), Offset(25f, 50.5f)), listOf(Offset(75f, 95f), Offset(100f, 5f))), lane?.lines)
        assertEquals(ChartMarker(50f, "14:02:05 modem", DeviceLevel.Neutral, 1), lane?.markers?.first())
        assertEquals(ChartGrid(72.5f, "25%"), lane?.grid?.first())
        assertEquals("14:00:30 · no reading", lane?.readings?.get(1))
        assertTrue(lane?.default == true)
        assertEquals(ChartSpan("Sortie 1", 0.1, 0.9), chart?.spans?.first())
        assertNull(parseFlightChart(null))
    }

    @Test
    fun `a cursor snaps to the nearest sample of the lane it is under`() {
        assertEquals(ChartSample(2, 100f), sampleAt(3, 0.9))
        assertEquals(ChartSample(1, 25f), sampleAt(5, 0.3))
        assertEquals(ChartSample(0, 0f), sampleAt(1, 0.7))
        assertNull(sampleAt(0, 0.5))
    }

    @Test
    fun `a drag inside a zoom narrows the zoom it is in`() {
        assertEquals(Zoom(0.2, 0.6), composeZoom(null, 0.6, 0.2))
        assertEquals(Zoom(0.5, 0.75), composeZoom(Zoom(0.5, 1.0), 0.0, 0.5))
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
