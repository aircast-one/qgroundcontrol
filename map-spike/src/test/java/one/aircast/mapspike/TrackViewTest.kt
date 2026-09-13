package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

private fun served(points: Int, dropped: Int = 0, available: Boolean = true): JSONObject {
    val listed = (0 until points).joinToString(",") {
        """{"latitude": ${47.0 + it * 0.001}, "longitude": 8.5}"""
    }
    return JSONObject(
        """{"class": "Track", "available": $available, "count": $points,
            "dropped": $dropped, "order": "oldestFirst", "points": [$listed]}""",
    )
}

class TrackViewTest {
    @Test
    fun `the trail is whatever the core recorded and nothing the head decided`() {
        val reading = trackReading(served(3))
        assertEquals(3, reading.points.size)
        assertEquals(47.0, reading.points.first().latitude, 1e-9)
    }

    @Test
    fun `one point is a position, not a line`() {
        assertFalse("a single fix draws nothing", trackDraws(trackReading(served(1))))
        assertTrue(trackDraws(trackReading(served(2))))
    }

    @Test
    fun `an unavailable track draws nothing however many points came with it`() {
        assertFalse(trackDraws(trackReading(served(5, available = false))))
        assertFalse(trackDraws(trackReading(null)))
    }

    @Test
    fun `the trim notice counts with the core's numbers, never a cap the head knows`() {
        assertNull("nothing dropped, nothing to say", trimmedNotice(trackReading(served(3))))
        assertEquals(
            "Trail trimmed — showing the last 500 positions of this flight.",
            trimmedNotice(trackReading(served(500, dropped = 212))),
        )
    }

    @Test
    fun `a point the map cannot plot is dropped rather than drawn at null island`() {
        val broken = JSONObject(
            """{"class": "Track", "available": true, "count": 2, "dropped": 0,
                "points": [{"latitude": 47.0, "longitude": 8.5}, {"latitude": null, "longitude": null}]}""",
        )
        assertEquals(1, trackReading(broken).points.size)
    }
}
