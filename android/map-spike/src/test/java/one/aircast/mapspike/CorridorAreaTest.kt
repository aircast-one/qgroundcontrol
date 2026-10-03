package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class CorridorAreaTest {

    @Test
    fun `a corridor shades its width outline like TransectStyleMapVisuals`() {
        val point = { lat: Double -> TrackPoint(lat, 8.0) }
        val corridor = Survey(index = 1, area = listOf(point(47.0), point(47.1)), transects = emptyList(), cameraShots = 0, kind = "corridor", shape = SHAPE_LINE, property = "corridorPolyline", outline = listOf(point(47.0), point(47.05), point(47.1)))
        val survey = Survey(index = 2, area = listOf(point(46.0), point(46.1), point(46.2)), transects = emptyList(), cameraShots = 0, kind = "survey", shape = SHAPE_AREA, property = "surveyAreaPolygon")
        assertEquals(corridor.outline, shadedArea(corridor))
        assertEquals(survey.area, shadedArea(survey))
        assertEquals(2, surveyAreaFeatures(listOf(corridor, survey)).features()!!.size)
        val tinted = surveyAreaFeatures(listOf(survey.copy(collides = true))).features()!!.single()
        assertEquals("a pattern that clips terrain is drawn in surveyPolygonTerrainCollision", true, tinted.getBooleanProperty(SURVEY_COLLISION))
        assertEquals(setOf(2), collidingItems(org.json.JSONObject("""{"collidingItems":[2]}""")))
    }
}
