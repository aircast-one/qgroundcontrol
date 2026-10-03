package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class InsertAfterSelectionTest {
    @Test
    fun `any selected mission item is current, as PlanView inserts after currentPlanViewVIIndex`() {
        assertEquals(4, missionItemIndex(MapHit.SurveyVertex(4, 2)))
        assertEquals(5, missionItemIndex(MapHit.ShapeCentre(fence = false, owner = 5)))
        assertEquals("a fence shape is not a mission item", null, missionItemIndex(MapHit.ShapeCentre(fence = true, owner = 5)))
        assertEquals(6, missionItemIndex(MapHit.LandingPlace(6, 0)))
        assertEquals(null, missionItemIndex(MapHit.Rally(1)))
    }
}
