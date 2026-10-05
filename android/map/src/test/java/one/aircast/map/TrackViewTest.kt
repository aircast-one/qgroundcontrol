package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

private fun served(
    points: Int,
    available: Boolean = true,
    from: Int = 0,
    count: Int = points,
    generation: Int = 1,
    vehicleId: Int = 1,
): JSONObject {
    val listed = (0 until points).joinToString(",") {
        """{"latitude": ${47.0 + (from + it) * 0.001}, "longitude": 8.5}"""
    }
    return JSONObject(
        """{"class": "Track", "available": $available, "count": $count, "vehicleId": $vehicleId,
            "generation": $generation, "from": $from, "order": "oldestFirst", "points": [$listed]}""",
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
    fun `a point the map cannot plot is not drawn but keeps its place in the trail`() {
        val broken = JSONObject(
            """{"class": "Track", "available": true, "count": 2, "from": 0,
                "points": [{"latitude": 47.0, "longitude": 8.5}, {"latitude": null, "longitude": null}]}""",
        )
        assertEquals(2, trackReading(broken).points.size)
        assertEquals(1, plottedTrack(trackReading(broken)).size)
    }

    @Test
    fun `a tail joins the trail already held and replaces its moved end point`() {
        val held = trackReading(served(300))
        val tail = trackReading(served(129, from = 299, count = 428))
        val merged = mergedTrack(held, tail)!!
        assertEquals(428, merged.points.size)
        assertEquals(0, merged.from)
        assertEquals(trackReading(served(428)).points, merged.points)
    }

    @Test
    fun `a tail the head cannot join asks for the whole trail`() {
        val held = trackReading(served(300))
        assertNull("points between the held trail and the tail were missed", mergedTrack(held, trackReading(served(128, from = 400, count = 528))))
        assertNull("a cleared trail is a new generation", mergedTrack(held, trackReading(served(128, from = 10, count = 138, generation = 2))))
        assertNull("another vehicle's trail", mergedTrack(held, trackReading(served(128, from = 10, count = 138, vehicleId = 2))))
        assertNull("nothing held yet", mergedTrack(null, trackReading(served(128, from = 10, count = 138))))
    }

    @Test
    fun `a short trail arrives whole in the tail`() {
        val tail = trackReading(served(5))
        assertEquals(tail, mergedTrack(trackReading(served(300, generation = 7)), tail))
    }
}
