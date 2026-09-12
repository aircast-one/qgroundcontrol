package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class VideoViewTest {

    private fun view(active: Int, multiple: Boolean, vararg statuses: String): JSONObject {
        val cameras = statuses.mapIndexed { slot, status ->
            """{"slot":$slot,"title":"Camera ${slot + 1}","status":"$status","connecting":false,
                "recording":false,"configured":${status.isNotBlank() && status != "No stream URL"}}"""
        }.joinToString(",")
        return JSONObject(
            """{"kind":"object","class":"Video","available":true,"decoding":false,
                "summary":"Not streaming.","activeSource":$active,"multipleSources":$multiple,
                "cameras":[$cameras]}""",
        )
    }

    @Test
    fun `the reading carries the core's summary and cameras`() {
        val reading = videoReading(view(1, true, "Streaming", "No stream URL"))!!

        assertEquals("Not streaming.", reading.summary)
        assertEquals(1, reading.activeSource)
        assertEquals(listOf("Camera 1", "Camera 2"), reading.cameras.map { it.title })
        assertEquals(listOf(true, false), reading.cameras.map { it.configured })
    }

    @Test
    fun `the source size is read only when the core reports a usable one`() {
        val decoding = view(0, false, "Streaming").put("sourceSize", JSONObject("""{"width":640,"height":480}"""))
        val zero = view(0, false, "Streaming").put("sourceSize", JSONObject("""{"width":0,"height":0}"""))

        assertEquals(SourceSize(640, 480), videoReading(decoding)!!.sourceSize)
        assertNull(videoReading(zero)!!.sourceSize)
        assertNull(videoReading(view(0, false, "Streaming"))!!.sourceSize)
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
    fun `a reply that is not the video view reads as nothing`() {
        assertNull(videoReading(null))
        assertNull(videoReading(JSONObject("""{"kind":"null"}""")))
    }

    @Test
    fun `a picker appears only when there is more than one stream to pick`() {
        assertTrue(switchableSources(videoReading(view(0, true, "Streaming", "No stream URL"))).isEmpty())
        assertTrue(switchableSources(videoReading(view(0, false, "Streaming", "Streaming"))).isEmpty())
        assertEquals(
            listOf(0, 1),
            switchableSources(videoReading(view(0, true, "Streaming", "Streaming"))).map { it.slot },
        )
    }
}

class VideoPanelVisibilityTest {
    @Test
    fun `a build that cannot show video should not hold map space explaining that`() {
        val off = videoReading(
            org.json.JSONObject(
                """{"class":"Video","available":false,"decoding":false,
                    "summary":"This build cannot show video.","activeSource":0,
                    "multipleSources":false}""",
            ),
        )

        org.junit.Assert.assertEquals(false, off?.available)
    }

    @Test
    fun `a configured source that is not decoding yet still earns its panel`() {
        val waiting = videoReading(
            org.json.JSONObject(
                """{"class":"Video","available":true,"decoding":false,
                    "summary":"Waiting for a stream.","activeSource":0,
                    "multipleSources":false}""",
            ),
        )

        org.junit.Assert.assertEquals(true, waiting?.available)
    }
}
