package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class CircleRingTest {
    private val centre = TrackPoint(41.0, 44.0)

    @Test
    fun `every point sits the requested distance from the centre`() {
        val ring = circleRing(centre, radiusMetres = 500.0, segments = 12)

        assertEquals(12, ring.size)
        ring.forEach { point ->
            assertEquals(500.0, metresBetween(centre, point), 1.0)
        }
    }

    @Test
    fun `the ring spreads all the way around`() {
        val ring = circleRing(centre, radiusMetres = 500.0, segments = 4)

        val north = ring.maxOf { it.latitude }
        val south = ring.minOf { it.latitude }
        val east = ring.maxOf { it.longitude }
        val west = ring.minOf { it.longitude }

        assertTrue(north > centre.latitude)
        assertTrue(south < centre.latitude)
        assertTrue(east > centre.longitude)
        assertTrue(west < centre.longitude)
    }

    @Test
    fun `a circle high on the globe still measures correctly`() {
        val arctic = TrackPoint(78.0, 15.0)
        val ring = circleRing(arctic, radiusMetres = 2_000.0, segments = 16)

        ring.forEach { point ->
            assertEquals(2_000.0, metresBetween(arctic, point), 2.0)
        }
    }

    @Test
    fun `a circle with no size or too few segments is not a ring`() {
        assertEquals(0, circleRing(centre, radiusMetres = 0.0).size)
        assertEquals(0, circleRing(centre, radiusMetres = -5.0).size)
        assertEquals(0, circleRing(centre, radiusMetres = 100.0, segments = 2).size)
    }

    @Test
    fun `circles become polygons that keep their index and inclusion`() {
        val polygons = circlesAsPolygons(
            listOf(
                FenceCircle(2, false, centre, 300.0),
                FenceCircle(3, true, centre, 0.0),
            ),
        )

        assertEquals(1, polygons.size)
        assertEquals(2, polygons.single().index)
        assertEquals(false, polygons.single().inclusion)
        assertTrue(polygons.single().vertices.size >= 3)
    }
}
