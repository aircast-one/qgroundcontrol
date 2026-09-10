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
