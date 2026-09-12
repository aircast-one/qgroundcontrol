package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SurveyFitTest {

    private val view = listOf(
        TrackPoint(42.0, 44.0), TrackPoint(42.0, 45.0),
        TrackPoint(41.0, 45.0), TrackPoint(41.0, 44.0),
    )

    private fun survey(corners: Int) = Survey(
        index = 2,
        area = (0 until corners).map { TrackPoint(41.5, 44.5) },
        transects = emptyList(),
        cameraShots = 0,
        kind = KIND_SURVEY,
        shape = SHAPE_AREA,
        property = "surveyAreaPolygon",
    )

    @Test
    fun `the fitted area sits inside what the operator can see, not flush against the edge`() {
        val inset = insetRing(view, 0.8)

        assertEquals(4, inset.size)
        assertTrue("north edge must come in from the top", inset.all { it.latitude < 42.0 })
        assertTrue("south edge must come up from the bottom", inset.all { it.latitude > 41.0 })
        assertEquals(41.9, inset[0].latitude, 1e-9)
        assertEquals(44.1, inset[0].longitude, 1e-9)
    }

    @Test
    fun `the inset keeps the centre where it was, so fitting does not walk the survey`() {
        val inset = insetRing(view, 0.8)

        assertEquals(41.5, inset.sumOf { it.latitude } / inset.size, 1e-9)
        assertEquals(44.5, inset.sumOf { it.longitude } / inset.size, 1e-9)
    }

    @Test
    fun `a shape with fewer corners than the view is refused rather than half written`() {
        assertFalse(fitSurveyArea(survey(3), insetRing(view, 0.8)))
        assertFalse(fitSurveyArea(survey(4), emptyList()))
    }

    @Test
    fun `nothing to inset yields nothing rather than a degenerate ring`() {
        assertEquals(emptyList<TrackPoint>(), insetRing(emptyList(), 0.8))
        assertEquals(emptyList<TrackPoint>(), insetRing(view.take(2), 0.8))
    }
}
