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
        assertEquals(ShapeTarget("$FENCE_POLYGONS.2", line = false), shapeTarget(2, null))
        assertNull(shapeTarget(null, null))
        val corridor = Survey(4, emptyList(), emptyList(), 0, "", "", CORRIDOR_PROPERTY)
        assertEquals(ShapeTarget("$PLAN_ITEMS.4.$CORRIDOR_PROPERTY", line = true), shapeTarget(null, corridor))
    }

    @Test
    fun `a file polygon is read and anything else is refused`() {
        val area = JSONObject("""{"shape":"polygon","error":"","points":[{"latitude":1.0,"longitude":2.0},{"latitude":3.0,"longitude":4.0}]}""")
        val polygon = ShapeTarget("p", line = false)
        val polyline = ShapeTarget("l", line = true)
        assertEquals(listOf(TrackPoint(1.0, 2.0), TrackPoint(3.0, 4.0)) to "", fileShape(area, polygon))
        assertEquals("No polylines found in file", fileShape(area, polyline).second)
        assertEquals("No polygons found in file", fileShape(JSONObject("""{"shape":"polyline","error":""}"""), polygon).second)
        assertEquals("bad coordinate: x", fileShape(JSONObject("""{"valid":false,"error":"bad coordinate: x"}"""), polygon).second)
        assertEquals("No polygons found in file", fileShape(null, polygon).second)
    }

    @Test
    fun `a trace closes once it is a polygon`() {
        assertEquals(small.take(2), traceOutline(small.take(2)))
        assertEquals(small.take(3) + small[0], traceOutline(small.take(3)))
        assertEquals(small.take(3), traceOutline(small.take(3), line = true))
    }

    @Test
    fun `a default line runs down the middle of the view`() {
        val (top, bottom) = defaultLine(small)
        assertEquals(8.5, top.longitude, 1e-9)
        assertEquals(47.4005, top.latitude, 1e-9)
        assertEquals(47.3995, bottom.latitude, 1e-9)
    }

    @Test
    fun `a shapefile picked with its prj is read through the shp`() {
        assertEquals("shp", mainShapeExtension(listOf("area.prj", "area.SHP")))
        assertEquals("kml", mainShapeExtension(listOf("area.kml")))
        assertNull(mainShapeExtension(emptyList()))
    }

    @Test
    fun `a typed circle radius is in the app distance units like QGC's Set Radius dialog`() {
        assertEquals(30.48, circleRadiusMetres("100", 0.3048)!!, 1e-9)
        assertEquals(12.5, circleRadiusMetres("12,5", 1.0)!!, 1e-9)
        assertNull(circleRadiusMetres("0", 0.3048))
        assertNull(circleRadiusMetres("wide", 1.0))
    }
}
