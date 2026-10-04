package one.aircast.mapspike

import androidx.compose.ui.geometry.Offset
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ObstacleOverlayTest {
    private fun overlay(ranges: List<Double>, increment: Double = 90.0, offset: Double = 0.0, max: Double = 40.0) =
        ObstacleOverlay(ranges, ranges.map { "%.2f".format(it) }, increment, offset, max)

    @Test
    fun `the overlay reads the core drawing and needs an increment and a range`() {
        val view = JSONObject("""{"ringIncrement":5.0,"ringOffset":0.0,"overlay":{"ranges":[3.2,655.35],"texts":["3.20","655.35"],"maxMetres":40.0}}""")
        assertEquals(ObstacleOverlay(listOf(3.2, 655.35), listOf("3.20", "655.35"), 5.0, 0.0, 40.0), obstacleOverlay(view))
        assertNull(obstacleOverlay(JSONObject("""{"ringIncrement":null,"overlay":{"ranges":[3.2],"texts":["3.20"],"maxMetres":40.0}}""")))
        assertNull(obstacleOverlay(JSONObject("""{"ringIncrement":5.0,"overlay":null}""")))
    }

    @Test
    fun `range index rounds up and turns with heading like rangeIdx`() {
        val ring = overlay(listOf(1.0, 2.0, 3.0, 4.0))
        assertEquals(0, rangeIndex(0.0, ring, 0.0))
        assertEquals(1, rangeIndex(10.0, ring, 0.0))
        assertEquals(3, rangeIndex(0.0, ring, 90.0))
        assertEquals(3, rangeIndex(90.0, overlay(listOf(1.0, 2.0, 3.0, 4.0), offset = 180.0), 0.0))
    }

    @Test
    fun `map points sit between the inner and outer radius and flip to true scale when zoomed in`() {
        val ring = overlay(listOf(40.0, 20.0, 0.0, 10.0))
        val shape = mapOverlayShape(ring, Offset(500f, 500f), 1000f, 100.0, 100f, 0.0, 0.0)
        assertEquals(90f, shape.gradientFrom)
        assertEquals(450f, shape.gradientTo)
        assertEquals(Offset(500f, 50f), shape.points[0].outer)
        assertEquals(Offset(770f, 500f), shape.points[1].outer)
        assertEquals(Offset(500f, 590f), shape.points[2].outer)
        val zoomed = mapOverlayShape(ring, Offset(500f, 500f), 1000f, 2.0, 100f, 0.0, 0.0)
        assertEquals(0f, zoomed.gradientFrom)
        assertEquals(2000f, zoomed.gradientTo)
    }

    @Test
    fun `map labels take the nearer of each pair and skip repeats within two metres`() {
        val ring = overlay(listOf(10.0, 9.0, 30.0, 11.0, 12.0, 30.0, 25.0, 50.0, 50.0), increment = 40.0)
        val shape = mapOverlayShape(ring, Offset(0f, 0f), 1000f, 100.0, 100f, 0.0, 0.0)
        assertEquals(listOf(9.0, 25.0), mapOverlayLabels(ring, shape.points).map { it.range })
    }

    @Test
    fun `video segments stop at the level that reaches the nearest range and label only it`() {
        val ring = overlay(List(16) { if (it == 0) 15.0 else 655.35 }, increment = 22.5)
        val (radii, segments) = videoOverlaySegments(ring, 1000f, showText = true)!!
        assertEquals(450f - 450f * 0.2f / 8f * 4 * 2, radii.first, 0.01f)
        val first = segments.filter { it.radFrom == 0.0 }
        assertEquals(3, first.size)
        assertEquals(listOf(null, null, "15.00"), first.map { it.label })
        assertEquals(15 + 3, segments.size)
        assertNull(videoOverlaySegments(overlay(listOf(1.0), max = 250.0), 1000f, showText = true))
    }
}
