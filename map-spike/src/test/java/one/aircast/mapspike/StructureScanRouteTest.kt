package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
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

class StructureScanLayersTest {
    private fun survey(layers: Int, span: String = "") =
        Survey(0, emptyList(), emptyList(), 0, "structure", "shape", "property", null, emptyList(), layers, span)

    @Test
    fun `stacked circuits are said in words, because on a map they sit exactly on each other`() {
        assertEquals("3 layers, one drawn", layersText(survey(3)))
    }

    @Test
    fun `the span is the core's sentence, so a feet rig does not read metres`() {
        assertEquals("3 layers, 12.0 m to 42.0 m, one drawn", layersText(survey(3, "12.0 m to 42.0 m")))
        assertEquals("3 layers, 39 ft to 138 ft, one drawn", layersText(survey(3, "39 ft to 138 ft")))
    }

    @Test
    fun `a span the core withheld leaves the count alone rather than inventing one`() {
        assertEquals("3 layers, one drawn", layersText(survey(3)))
    }

    @Test
    fun `a single layer says nothing, because the loop drawn is the whole mission`() {
        assertNull(layersText(survey(1)))
        assertNull(layersText(survey(0)))
        assertNull(layersText(null))
    }
}

class PatternNameTest {
    private fun item(index: Int, name: String) =
        MissionItem(index, index, 41.0, 44.0, name, false)

    @Test
    fun `a structure scan is not called a survey, because the core already named it`() {
        val items = listOf(item(2, "Structure Scan"), item(3, "Corridor Scan"))

        assertEquals("Structure Scan", patternName(2, items))
        assertEquals("Corridor Scan", patternName(3, items))
    }

    @Test
    fun `an item the list does not hold falls back to a word that is true of all three`() {
        assertEquals("pattern", patternName(9, emptyList()))
        assertEquals("pattern", patternName(2, listOf(item(2, ""))))
    }
}
