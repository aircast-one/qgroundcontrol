package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CenterMenuTest {
    @Test
    fun `a typed coordinate centres only when it is on the globe`() {
        assertEquals(TrackPoint(47.5, 8.25), parsedCoordinate(" 47.5 ", "8.25"))
        assertNull(parsedCoordinate("91", "8"))
        assertNull(parsedCoordinate("47", "east"))
    }

    @Test
    fun `mission fit uses the mission items that have a position`() {
        val items = listOf(
            MissionItem(0, 0, 47.0, 8.0, "Home", false),
            MissionItem(1, 1, Double.NaN, Double.NaN, "Delay", false),
            MissionItem(2, 2, 47.1, 8.1, "Waypoint", false),
        )
        assertEquals(listOf(TrackPoint(47.0, 8.0), TrackPoint(47.1, 8.1)), missionFitPoints(items))
    }

    @Test
    fun `a selected corner's position comes from its fence or survey`() {
        val fence = FencePolygon(3, true, listOf(TrackPoint(1.0, 2.0), TrackPoint(3.0, 4.0)))
        val survey = Survey(5, listOf(TrackPoint(5.0, 6.0)), emptyList(), 0, "survey", "polygon", "surveyAreaPolygon")
        assertEquals(TrackPoint(3.0, 4.0), cornerPosition(MapHit.FenceVertex(3, 1), listOf(fence), listOf(survey)))
        assertEquals(TrackPoint(5.0, 6.0), cornerPosition(MapHit.SurveyVertex(5, 0), listOf(fence), listOf(survey)))
        assertNull(cornerPosition(MapHit.FenceVertex(3, 9), listOf(fence), listOf(survey)))
    }
}
