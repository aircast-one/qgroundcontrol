package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceRecordingsTest {
    private val segment = RecordingSegment("2026-10-18T10:00:00+04:00", "18 Oct 10:00:00", "10 min", "1.0 MB", "/v", open = false, noTelemetry = false)

    @Test
    fun `reads the recordings page the device serves`() {
        val view = parseRecordings(
            JSONObject(
                """
                {
                  "limit": {"value": "8589934592", "options": [{"value": "2147483648", "label": "2 GB"}], "used": "2.0 KB used"},
                  "cameras": [{"name": "cam2", "recording": true, "note": "Recording · 2.0 KB on disk"}],
                  "recorded": [{"name": "cam2", "status": {"level": 1, "text": "storage full"}, "summary": "1 recording", "full": true, "usage": "2.0 KB on this camera", "segments": [{"at": "a", "started": "s", "size": "2.0 KB", "url": "/v", "open": true}]}]
                }
                """,
            ),
        )
        assertEquals(listOf(DeviceOption("2147483648", "2 GB")), view?.limit?.options)
        assertEquals(RecordCamera("cam2", true, "Recording · 2.0 KB on disk"), view?.cameras?.first())
        assertTrue(view?.recorded?.first()?.full == true)
        assertEquals(DeviceBadge(1, "storage full"), view?.recorded?.first()?.status)
        assertEquals(RecordingSegment("a", "s", "", "2.0 KB", "/v", open = true, noTelemetry = false), view?.recorded?.first()?.segments?.first())
        assertNull(parseRecordings(JSONObject("""{"cameras":[]}""")))
    }

    @Test
    fun `only a smaller limit asks before it is saved`() {
        assertTrue(lowersLimit("8589934592", "2147483648"))
        assertFalse(lowersLimit("8589934592", "17179869184"))
        assertFalse(lowersLimit("", "2147483648"))
    }

    @Test
    fun `a segment is deleted by its start, escaped`() {
        assertEquals("/api/recordings/cam2?start=2026-10-18T10%3A00%3A00%2B04%3A00", segmentPath("cam2", segment))
        assertEquals("/api/streams/cam2/record", recordPath("cam2"))
    }

    @Test
    fun `a segment says what it is missing and when it can be had`() {
        assertEquals(listOf("10 min", "1.0 MB"), segmentDetail(segment))
        assertEquals(listOf("1.0 MB", "no flight data", "downloadable once it closes"), segmentDetail(segment.copy(duration = "", noTelemetry = true, open = true)))
    }
}
