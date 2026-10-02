package one.aircast.mapspike

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DragAllowedTest {
    @Test
    fun `only the selected item on the active layer can be dragged`() {
        assertTrue(dragAllowed(MapHit.Waypoint(2), MapHit.Waypoint(2), PlanLayer.Mission))
        assertFalse("MissionItemMapVisualBase only drags the current item", dragAllowed(MapHit.Waypoint(3), MapHit.Waypoint(2), PlanLayer.Mission))
        assertTrue("a selected survey's corners move", dragAllowed(MapHit.SurveyVertex(2, 1), MapHit.Waypoint(2), PlanLayer.Mission))
        assertFalse("a rally point is not dragged from the mission layer", dragAllowed(MapHit.Rally(0), MapHit.Rally(0), PlanLayer.Mission))
        assertTrue(dragAllowed(MapHit.Rally(0), MapHit.Rally(0), PlanLayer.Rally))
        assertFalse("nothing selected, nothing dragged", dragAllowed(MapHit.FenceVertex(0, 1), null, PlanLayer.Fence))
        assertTrue(dragAllowed(MapHit.FenceVertex(0, 1), MapHit.FenceVertex(0, 0), PlanLayer.Fence))
    }
}
