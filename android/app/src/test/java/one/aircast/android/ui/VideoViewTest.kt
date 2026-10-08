package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class VideoViewTest {

    private fun view(active: Int, vararg statuses: String): JSONObject {
        val cameras = statuses.mapIndexed { slot, status ->
            """{"slot":$slot,"title":"Camera ${slot + 1}","status":"$status","connecting":false,
                "recording":false,"configured":${status.isNotBlank() && status != "No stream URL"}}"""
        }.joinToString(",")
        return JSONObject(
            """{"kind":"object","class":"Video","available":true,"decoding":false,
                "summary":"Not streaming.","activeSource":$active,
                "cameras":[$cameras]}""",
        )
    }

    @Test
    fun `the reading carries the core's summary and cameras`() {
        val reading = videoReading(view(1, "Streaming", "No stream URL"))!!

        assertEquals("Not streaming.", reading.summary)
        assertEquals(1, reading.activeSource)
        assertEquals(listOf("Streaming", "No stream URL"), reading.cameras.map { it.status })
        assertEquals(listOf(true, false), reading.cameras.map { it.configured })
    }

    @Test
    fun `a stream the core switched off reads as off, and one it does not mention as on`() {
        assertEquals(false, videoReading(view(0, "Streaming").put("streamEnabled", false))!!.streamEnabled)
        assertEquals(true, videoReading(view(0, "Streaming"))!!.streamEnabled)
    }

    @Test
    fun `the source size is read only when the core reports a usable one`() {
        val decoding = view(0, "Streaming").put("sourceSize", JSONObject("""{"width":640,"height":480}"""))
        val zero = view(0, "Streaming").put("sourceSize", JSONObject("""{"width":0,"height":0}"""))

        assertEquals(SourceSize(640, 480), videoReading(decoding)!!.sourceSize)
        assertNull(videoReading(zero)!!.sourceSize)
        assertNull(videoReading(view(0, "Streaming"))!!.sourceSize)
    }

    @Test
    fun `the picture is letterboxed inside the surface it is painted on`() {
        val wide = paintedRect(1080.0, 1770.0, SourceSize(640, 480))
        assertEquals(1080.0, wide.width, 0.001)
        assertEquals(810.0, wide.height, 0.001)
        assertEquals(0.0, wide.left, 0.001)
        assertEquals(480.0, wide.top, 0.001)

        val tall = paintedRect(400.0, 200.0, SourceSize(480, 640))
        assertEquals(150.0, tall.width, 0.001)
        assertEquals(200.0, tall.height, 0.001)
        assertEquals(125.0, tall.left, 0.001)
    }

    @Test
    fun `an unknown source size paints the whole surface`() {
        val whole = paintedRect(300.0, 200.0, null)

        assertEquals(0.0, whole.left, 0.001)
        assertEquals(300.0, whole.width, 0.001)
        assertEquals(200.0, whole.height, 0.001)
    }

    @Test
    fun `the picture-in-picture plays only while a thumbnail surface is on screen`() {
        val sent = mutableListOf<Boolean>()
        val surfaces = PipSurfaces { sent += it }
        surfaces.created()
        surfaces.destroyed()
        assertEquals(listOf(true, false), sent)
    }

    @Test
    fun `a thumbnail that reappears before the old one is torn down keeps the picture-in-picture playing`() {
        val sent = mutableListOf<Boolean>()
        val surfaces = PipSurfaces { sent += it }
        surfaces.created()
        surfaces.created()
        surfaces.destroyed()
        assertEquals(listOf(true, true, true), sent)
        surfaces.destroyed()
        assertEquals(false, sent.last())
    }

    @Test
    fun `a reply that is not the video view reads as nothing`() {
        assertNull(videoReading(null))
        assertNull(videoReading(JSONObject("""{"kind":"null"}""")))
    }
}

class VideoPanelVisibilityTest {
    @Test
    fun `a source that is not configured should not hold map space explaining that`() {
        val off = videoReading(
            org.json.JSONObject(
                """{"class":"Video","available":false,"decoding":false,
                    "summary":"No stream URL is set.","activeSource":0}""",
            ),
        )

        assertEquals(false, off?.available)
    }

    @Test
    fun `a configured source that is not decoding yet still earns its panel`() {
        val waiting = videoReading(
            org.json.JSONObject(
                """{"class":"Video","available":true,"decoding":false,
                    "summary":"Waiting for a stream.","activeSource":0}""",
            ),
        )

        assertEquals(true, waiting?.available)
    }
}
