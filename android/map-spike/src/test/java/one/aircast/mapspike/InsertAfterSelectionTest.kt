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

    @Test
    fun `an item not ready to send shows a question mark seal like MissionItemEditor`() {
        val item = MissionItem(2, 2, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint", commandId = 16)
        assertEquals("2", itemRows(listOf(item)).single().number)
        assertEquals(NOT_READY_SEAL, itemRows(listOf(item.copy(readyForSave = false))).single().number)
    }
}
