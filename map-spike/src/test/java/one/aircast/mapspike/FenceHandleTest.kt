package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class FenceHandleTest {
    private fun polygon(index: Int, vararg vertices: Pair<Double, Double>) =
        FencePolygon(index, true, vertices.map { TrackPoint(it.first, it.second) })

    @Test
    fun `every vertex of every polygon gets a handle`() {
        val features = fenceHandleFeatures(
            listOf(
                polygon(0, 41.0 to 44.0, 41.0 to 44.1, 41.1 to 44.1),
                polygon(1, 42.0 to 45.0, 42.0 to 45.1, 42.1 to 45.1, 42.1 to 45.0),
            ),
        )

        assertEquals(7, features.features()?.size)
    }

    @Test
    fun `a handle carries the polygon and vertex it belongs to`() {
        val features = fenceHandleFeatures(
            listOf(polygon(3, 41.0 to 44.0, 41.0 to 44.1, 41.1 to 44.1)),
        )
        val second = features.features()!![1]

        assertEquals(3, second.getNumberProperty(POLYGON_INDEX_PROPERTY).toInt())
        assertEquals(1, second.getNumberProperty(VERTEX_INDEX_PROPERTY).toInt())
    }

    @Test
    fun `no polygons means no handles`() {
        assertEquals(0, fenceHandleFeatures(emptyList()).features()?.size)
    }
}
