package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class FenceHandleTest {
    private fun polygon(index: Int, vararg vertices: Pair<Double, Double>) =
        FencePolygon(index, true, vertices.map { TrackPoint(it.first, it.second) })

    private fun survey(index: Int, vararg vertices: Pair<Double, Double>) =
        Survey(index, vertices.map { TrackPoint(it.first, it.second) }, emptyList(), 0)

    @Test
    fun `every vertex of every polygon gets a handle`() {
        val features = vertexHandleFeatures(
            listOf(
                polygon(0, 41.0 to 44.0, 41.0 to 44.1, 41.1 to 44.1),
                polygon(1, 42.0 to 45.0, 42.0 to 45.1, 42.1 to 45.1, 42.1 to 45.0),
            ),
            emptyList(),
        )

        assertEquals(7, features.features()?.size)
    }

    @Test
    fun `a handle carries the polygon and vertex it belongs to`() {
        val features = vertexHandleFeatures(
            listOf(polygon(3, 41.0 to 44.0, 41.0 to 44.1, 41.1 to 44.1)),
            emptyList(),
        )
        val second = features.features()!![1]

        assertEquals(HANDLE_KIND_FENCE, second.getStringProperty(HANDLE_KIND_PROPERTY))
        assertEquals(3, second.getNumberProperty(POLYGON_INDEX_PROPERTY).toInt())
        assertEquals(1, second.getNumberProperty(VERTEX_INDEX_PROPERTY).toInt())
    }

    @Test
    fun `a survey area gets handles marked as its own kind`() {
        val features = vertexHandleFeatures(
            emptyList(),
            listOf(survey(5, 41.0 to 44.0, 41.0 to 44.1, 41.1 to 44.1)),
        )
        val first = features.features()!!.first()

        assertEquals(3, features.features()?.size)
        assertEquals(HANDLE_KIND_SURVEY, first.getStringProperty(HANDLE_KIND_PROPERTY))
        assertEquals(5, first.getNumberProperty(POLYGON_INDEX_PROPERTY).toInt())
    }

    @Test
    fun `fence and survey handles share one collection without colliding`() {
        val features = vertexHandleFeatures(
            listOf(polygon(0, 41.0 to 44.0, 41.0 to 44.1, 41.1 to 44.1)),
            listOf(survey(0, 42.0 to 45.0, 42.0 to 45.1, 42.1 to 45.1)),
        )
        val kinds = features.features()!!.map { it.getStringProperty(HANDLE_KIND_PROPERTY) }

        assertEquals(6, features.features()?.size)
        assertEquals(3, kinds.count { it == HANDLE_KIND_FENCE })
        assertEquals(3, kinds.count { it == HANDLE_KIND_SURVEY })
    }

    @Test
    fun `nothing to edit means no handles`() {
        assertEquals(0, vertexHandleFeatures(emptyList(), emptyList()).features()?.size)
    }

    @Test
    fun `a circle gets one handle at its centre`() {
        val features = vertexHandleFeatures(
            emptyList(),
            emptyList(),
            listOf(FenceCircle(2, true, TrackPoint(41.0, 44.0), 150.0)),
        )
        val handle = features.features()!!.single()

        assertEquals(HANDLE_KIND_CIRCLE, handle.getStringProperty(HANDLE_KIND_PROPERTY))
        assertEquals(2, handle.getNumberProperty(POLYGON_INDEX_PROPERTY).toInt())
        assertEquals(0, handle.getNumberProperty(VERTEX_INDEX_PROPERTY).toInt())
    }

    @Test
    fun `all three kinds of handle coexist and stay distinguishable`() {
        val features = vertexHandleFeatures(
            listOf(polygon(0, 41.0 to 44.0, 41.0 to 44.1, 41.1 to 44.1)),
            listOf(survey(0, 42.0 to 45.0, 42.0 to 45.1, 42.1 to 45.1)),
            listOf(FenceCircle(0, true, TrackPoint(43.0, 46.0), 150.0)),
        )
        val kinds = features.features()!!.map { it.getStringProperty(HANDLE_KIND_PROPERTY) }

        assertEquals(7, features.features()?.size)
        assertEquals(3, kinds.count { it == HANDLE_KIND_FENCE })
        assertEquals(3, kinds.count { it == HANDLE_KIND_SURVEY })
        assertEquals(1, kinds.count { it == HANDLE_KIND_CIRCLE })
    }
}
