package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ShotPointsTest {

    private fun video(points: String) = JSONObject(
        """{"kind":"object","class":"Video","available":true,"decoding":true,"shotPoints":$points}""",
    )

    @Test
    fun `every photo the vehicle reported gets a place on the map`() {
        val points = shotPoints(
            video("""[{"latitude":47.397,"longitude":8.545},{"latitude":47.398,"longitude":8.546}]"""),
        )

        assertEquals(
            "the core counted the photos and said where each was taken; the head reported the " +
                "count and drew none of them, so a survey's coverage gaps were invisible on a " +
                "screen QGC marks every trigger point on",
            2,
            points.size,
        )
        assertEquals(47.397, points[0].latitude, 1e-9)
        assertEquals(8.546, points[1].longitude, 1e-9)
    }

    @Test
    fun `a vehicle that has taken no photos draws nothing`() {
        assertEquals(emptyList<TrackPoint>(), shotPoints(video("[]")))
        assertEquals(emptyList<TrackPoint>(), shotPoints(JSONObject("""{"kind":"object"}""")))
        assertEquals(emptyList<TrackPoint>(), shotPoints(null))
    }

    @Test
    fun `a point without a usable coordinate is dropped rather than plotted at zero`() {
        val points = shotPoints(
            video("""[{"latitude":47.397,"longitude":8.545},{"longitude":8.546},{"latitude":null,"longitude":null}]"""),
        )

        assertEquals(1, points.size)
        assertTrue(points.single().latitude > 47.0)
    }

    @Test
    fun `the features handed to the map carry longitude first, as GeoJSON wants it`() {
        val collection = shotFeatures(listOf(TrackPoint(47.397, 8.545)))
        val json = collection.toJson()

        assertTrue("GeoJSON is [longitude, latitude] and swapping them lands the shot in Somalia", json.contains("8.545"))
        assertEquals(1, collection.features()?.size)
        assertTrue(json.indexOf("8.545") < json.indexOf("47.397"))
    }
}
