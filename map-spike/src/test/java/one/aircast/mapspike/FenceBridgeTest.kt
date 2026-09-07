package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class FenceBridgeTest {
    private fun vertex(latitude: Double, longitude: Double) =
        """{"latitude":$latitude,"longitude":$longitude}"""

    private fun polygon(vararg vertices: String, inclusion: Boolean = true) =
        """{"inclusion":$inclusion,"path":[${vertices.joinToString(",")}]}"""

    private fun model(vararg elements: String) =
        JSONObject("""{"kind":"object","elements":[${elements.joinToString(",")}]}""")

    private val square = arrayOf(
        vertex(41.0, 44.0),
        vertex(41.0, 44.1),
        vertex(41.1, 44.1),
        vertex(41.1, 44.0),
    )

    @Test
    fun `a closed polygon keeps its vertices and inclusion`() {
        val polygons = fencePolygons(model(polygon(*square, inclusion = false)))

        assertEquals(1, polygons.size)
        assertEquals(4, polygons.single().vertices.size)
        assertFalse(polygons.single().inclusion)
        assertEquals(TrackPoint(41.0, 44.0), polygons.single().vertices.first())
    }

    @Test
    fun `a polygon that cannot close is not drawn`() {
        val polygons = fencePolygons(
            model(
                polygon(vertex(41.0, 44.0), vertex(41.0, 44.1)),
                polygon(*square),
            ),
        )

        assertEquals(1, polygons.size)
    }

    @Test
    fun `unusable vertices are dropped and can sink a polygon`() {
        val polygons = fencePolygons(
            model(polygon(vertex(41.0, 44.0), vertex(0.0, 0.0), vertex(41.1, 44.1))),
        )

        assertEquals(0, polygons.size)
    }

    private fun radiusFact(value: Double) = """{"name":"Radius","value":$value}"""

    @Test
    fun `a circle reads its radius from the fact list`() {
        val good = """{"inclusion":true,"center":${vertex(41.0, 44.0)},"facts":[${radiusFact(136.0)}]}"""

        val circles = fenceCircles(model(good))

        assertEquals(1, circles.size)
        assertEquals(136.0, circles.single().radius, 1e-9)
        assertTrue(circles.single().inclusion)
    }

    @Test
    fun `circles without a centre or a usable radius are dropped`() {
        val noRadius = """{"inclusion":true,"center":${vertex(41.0, 44.0)},"facts":[${radiusFact(0.0)}]}"""
        val noFacts = """{"inclusion":true,"center":${vertex(41.0, 44.0)}}"""
        val noCentre = """{"inclusion":true,"facts":[${radiusFact(120.0)}]}"""
        val otherFact = """{"center":${vertex(41.0, 44.0)},"facts":[{"name":"Altitude","value":50.0}]}"""

        assertEquals(0, fenceCircles(model(noRadius, noFacts, noCentre, otherFact)).size)
    }

    @Test
    fun `rally points read their coordinate`() {
        val points = rallyPoints(
            model(
                """{"coordinate":${vertex(41.2, 44.2)}}""",
                """{"coordinate":${vertex(0.0, 0.0)}}""",
                """{}""",
            ),
        )

        assertEquals(1, points.size)
        assertEquals(41.2, points.single().latitude, 1e-9)
        assertEquals(0, points.single().index)
    }

    @Test
    fun `an absent model yields nothing`() {
        assertEquals(0, fencePolygons(null).size)
        assertEquals(0, fenceCircles(null).size)
        assertEquals(0, rallyPoints(null).size)
    }
}
