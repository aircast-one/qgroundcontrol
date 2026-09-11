package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SelectionValidityTest {
    private fun item(index: Int) =
        MissionItem(index, index + 1, 41.0, 44.0, "Waypoint", false, Double.NaN)

    private fun survives(selected: MapHit?, items: List<MissionItem> = emptyList()) =
        selectionSurvives(selected, items, emptyList(), emptyList(), emptyList(), emptyList())

    @Test
    fun `no selection always survives`() {
        assertTrue(survives(null))
    }

    @Test
    fun `a waypoint that is still there survives`() {
        assertTrue(survives(MapHit.Waypoint(2), listOf(item(1), item(2))))
    }

    @Test
    fun `a waypoint the plan no longer holds does not`() {
        assertFalse(survives(MapHit.Waypoint(2), listOf(item(0), item(1))))
        assertFalse(survives(MapHit.Waypoint(0), emptyList()))
    }

    @Test
    fun `a circle selected by either handle follows the circle`() {
        val circles = listOf(FenceCircle(1, true, TrackPoint(41.0, 44.0), 100.0))
        val gone = emptyList<FenceCircle>()

        assertTrue(selectionSurvives(MapHit.Circle(1), emptyList(), emptyList(), circles, emptyList(), emptyList()))
        assertTrue(selectionSurvives(MapHit.CircleCentre(1), emptyList(), emptyList(), circles, emptyList(), emptyList()))
        assertFalse(selectionSurvives(MapHit.Circle(1), emptyList(), emptyList(), gone, emptyList(), emptyList()))
    }

    @Test
    fun `a vertex outside the shape it names does not survive`() {
        val polygons = listOf(FencePolygon(0, true, listOf(TrackPoint(41.0, 44.0), TrackPoint(41.1, 44.1))))

        assertTrue(selectionSurvives(MapHit.FenceVertex(0, 1), emptyList(), polygons, emptyList(), emptyList(), emptyList()))
        assertFalse(selectionSurvives(MapHit.FenceVertex(0, 5), emptyList(), polygons, emptyList(), emptyList(), emptyList()))
    }

    @Test
    fun `a rally point that has gone does not survive`() {
        val rally = listOf(RallyPoint(3, 41.0, 44.0))

        assertTrue(selectionSurvives(MapHit.Rally(3), emptyList(), emptyList(), emptyList(), rally, emptyList()))
        assertFalse(selectionSurvives(MapHit.Rally(4), emptyList(), emptyList(), emptyList(), rally, emptyList()))
    }
}

class SelectedSurveyTest {
    private fun survey(index: Int) =
        Survey(index, listOf(TrackPoint(41.0, 44.0)), emptyList(), 0, KIND_SURVEY, SHAPE_AREA, "surveyAreaPolygon")

    private val surveys = listOf(survey(1), survey(4))

    @Test
    fun `the selected survey is the one acted on, not the first`() {
        assertEquals(4, selectedSurvey(MapHit.SurveyVertex(4, 0), surveys)!!.index)
        assertEquals(1, selectedSurvey(MapHit.SurveyVertex(1, 0), surveys)!!.index)
    }

    @Test
    fun `nothing selected means no survey to act on`() {
        assertNull(selectedSurvey(null, surveys))
        assertNull(selectedSurvey(MapHit.Waypoint(1), surveys))
    }

    @Test
    fun `a selection naming a survey that is gone yields nothing`() {
        assertNull(selectedSurvey(MapHit.SurveyVertex(9, 0), surveys))
    }
}

class CornerRemovalTest {
    private fun polygon(corners: Int) = FencePolygon(
        index = 0,
        inclusion = true,
        vertices = (0 until corners).map { TrackPoint(it.toDouble(), it.toDouble()) },
    )

    @Test
    fun `a triangle cannot lose a corner, because the core refuses to trash the polygon`() {
        assertFalse(cornerRemovable(polygon(3)))
    }

    @Test
    fun `a quadrilateral can lose one`() {
        assertTrue(cornerRemovable(polygon(4)))
    }

    @Test
    fun `a polygon that is not there offers nothing`() {
        assertFalse(cornerRemovable(null))
    }

    @Test
    fun `a degenerate polygon is never offered a removal`() {
        assertFalse(cornerRemovable(polygon(0)))
        assertFalse(cornerRemovable(polygon(2)))
    }
}
