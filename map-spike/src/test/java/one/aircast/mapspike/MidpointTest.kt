package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

class MidpointTest {

    private val square = FencePolygon(
        index = 0,
        inclusion = true,
        vertices = listOf(
            TrackPoint(41.70, 44.82), TrackPoint(41.70, 44.83),
            TrackPoint(41.71, 44.83), TrackPoint(41.71, 44.82),
        ),
        midpoints = listOf(
            TrackPoint(41.700, 44.825), TrackPoint(41.705, 44.830),
            TrackPoint(41.710, 44.825), TrackPoint(41.705, 44.820),
        ),
    )

    @Test
    fun `every segment offers somewhere to add a corner`() {
        val features = midpointFeatures(listOf(square)).features()!!

        assertEquals(4, features.size)
        assertEquals(
            listOf(0, 1, 2, 3),
            features.map { it.getNumberProperty(VERTEX_INDEX_PROPERTY).toInt() },
        )
        assertEquals(
            listOf(0, 0, 0, 0),
            features.map { it.getNumberProperty(POLYGON_INDEX_PROPERTY).toInt() },
        )
    }

    @Test
    fun `a polygon the core gave no midpoints for offers none`() {
        assertEquals(0, midpointFeatures(listOf(square.copy(midpoints = emptyList()))).features()!!.size)
        assertEquals(0, midpointFeatures(emptyList()).features()!!.size)
    }

    @Test
    fun `a midpoint is a button, so it never writes a position`() {
        assertFalse(writeMove(MapHit.Midpoint(0, 2), 41.0, 44.0, emptyList()))
    }

    @Test
    fun `a midpoint is never a lasting selection`() {
        assertFalse(
            selectionSurvives(
                MapHit.Midpoint(0, 2),
                emptyList(), listOf(square), emptyList(), emptyList(), emptyList(),
            ),
        )
    }
}
