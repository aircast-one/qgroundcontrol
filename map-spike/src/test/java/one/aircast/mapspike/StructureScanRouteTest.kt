package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class StructureScanRouteTest {
    private fun survey(
        transects: List<TrackPoint> = emptyList(),
        loop: List<TrackPoint> = emptyList(),
    ) = Survey(0, emptyList(), transects, 0, "structure", "shape", "property", null, loop, 3)

    private val square = listOf(
        TrackPoint(41.0, 44.0),
        TrackPoint(41.0, 44.1),
        TrackPoint(41.1, 44.1),
    )

    @Test
    fun `a structure scan flies a closed loop, so the route returns to where it started`() {
        val route = flownRoute(survey(loop = square))

        assertEquals(square.size + 1, route.size)
        assertEquals(route.first(), route.last())
    }

    @Test
    fun `a loop already closed is not closed twice`() {
        val closed = square + square.first()

        assertEquals(closed, flownRoute(survey(loop = closed)))
    }

    @Test
    fun `a survey mows, so its transects are the route and are left open`() {
        val route = flownRoute(survey(transects = square))

        assertEquals(square, route)
    }

    @Test
    fun `an item with neither draws no route rather than a degenerate one`() {
        assertEquals(emptyList<TrackPoint>(), flownRoute(survey()))
        assertEquals(emptyList<TrackPoint>(), flownRoute(survey(loop = square.take(1))))
    }
}
