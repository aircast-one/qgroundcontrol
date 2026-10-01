package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.json.JSONObject
import org.junit.Test

class PolygonToolsTest {
    private val small = listOf(
        TrackPoint(47.401, 8.499), TrackPoint(47.401, 8.501), TrackPoint(47.399, 8.501), TrackPoint(47.399, 8.499),
    )

    @Test
    fun `rectangle fills three quarters of the view`() {
        val corners = defaultRectangle(small)
        assertEquals(4, corners.size)
        val width = metresBetween(small[0], small[1])
        assertEquals(width * 0.75, metresBetween(corners[0], corners[1]), 1.0)
        assertTrue(corners[0].latitude > corners[3].latitude && corners[0].longitude < corners[1].longitude)
    }

    @Test
    fun `a zoomed out view caps the shape at three kilometres`() {
        val wide = listOf(TrackPoint(48.0, 8.0), TrackPoint(48.0, 9.0), TrackPoint(47.0, 9.0), TrackPoint(47.0, 8.0))
        assertEquals(3000.0, metresBetween(defaultRectangle(wide)[0], defaultRectangle(wide)[1]), 5.0)
    }

    @Test
    fun `circle has sixteen corners at the smaller half extent`() {
        val ring = defaultCircle(small)
        assertEquals(16, ring.size)
        val centre = TrackPoint(47.4, 8.5)
        val half = minOf(metresBetween(small[0], small[1]), metresBetween(small[0], small[3])) * 0.75 / 2
        ring.forEach { assertEquals(half, metresBetween(centre, it), 1.0) }
    }

    @Test
    fun `no view no shape`() {
        assertTrue(defaultRectangle(emptyList()).isEmpty())
        assertTrue(defaultCircle(small.take(3)).isEmpty())
    }

    @Test
    fun `shape path names the fence or the survey area`() {
        assertEquals("$FENCE_POLYGONS.2", shapePath(2, null))
        assertNull(shapePath(null, null))
    }

    @Test
    fun `a file polygon is read and anything else is refused`() {
        val area = JSONObject("""{"shape":"polygon","error":"","points":[{"latitude":1.0,"longitude":2.0},{"latitude":3.0,"longitude":4.0}]}""")
        assertEquals(listOf(TrackPoint(1.0, 2.0), TrackPoint(3.0, 4.0)) to "", filePolygon(area))
        assertEquals(NO_POLYGON_IN_FILE, filePolygon(JSONObject("""{"shape":"polyline","error":""}""")).second)
        assertEquals("bad coordinate: x", filePolygon(JSONObject("""{"valid":false,"error":"bad coordinate: x"}""")).second)
        assertEquals(NO_POLYGON_IN_FILE, filePolygon(null).second)
    }
}
