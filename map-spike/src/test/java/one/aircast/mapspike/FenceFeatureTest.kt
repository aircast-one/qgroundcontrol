package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FenceFeatureTest {
    private fun ring(index: Int) = FencePolygon(
        index,
        true,
        listOf(TrackPoint(41.0, 44.0), TrackPoint(41.0, 44.1), TrackPoint(41.1, 44.1)),
    )

    @Test
    fun `a circle carries the index that finds it again`() {
        val features = fenceFeatures(emptyList(), listOf(ring(4)))

        assertEquals(1, features.features()?.size)
        assertEquals(4, features.features()!!.single().getNumberProperty(CIRCLE_INDEX_PROPERTY).toInt())
    }

    @Test
    fun `a polygon carries no circle index so it cannot be mistaken for one`() {
        val features = fenceFeatures(listOf(ring(0)), emptyList())

        assertEquals(1, features.features()?.size)
        assertNull(features.features()!!.single().getNumberProperty(CIRCLE_INDEX_PROPERTY))
    }

    @Test
    fun `polygons and circles keep separate index spaces in one collection`() {
        val features = fenceFeatures(listOf(ring(0)), listOf(ring(0)))
        val tagged = features.features()!!.filter { it.getNumberProperty(CIRCLE_INDEX_PROPERTY) != null }

        assertEquals(2, features.features()?.size)
        assertEquals(1, tagged.size)
    }
}
