package one.aircast.map

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
        assertTrue("a selected polygon's radius handle is its own", dragAllowed(MapHit.ShapeRadius(fence = true, owner = 1), MapHit.FenceVertex(1, 0), PlanLayer.Fence))
    }

    @Test
    fun `a midpoint splits only the selected shape`() {
        assertTrue(dragAllowed(MapHit.Midpoint("$FENCE_POLYGONS.1", "split", 0), MapHit.FenceVertex(1, 0), PlanLayer.Fence))
        assertFalse(dragAllowed(MapHit.Midpoint("$FENCE_POLYGONS.1", "split", 0), MapHit.FenceVertex(0, 0), PlanLayer.Fence))
        assertTrue(dragAllowed(MapHit.Midpoint("$PLAN_ITEMS.2.surveyAreaPolygon", "split", 0), MapHit.Waypoint(2), PlanLayer.Mission))
        assertFalse(dragAllowed(MapHit.Midpoint("$PLAN_ITEMS.2.surveyAreaPolygon", "split", 0), MapHit.Waypoint(3), PlanLayer.Mission))
        assertTrue(dragAllowed(MapHit.Midpoint(MISSION_SPLIT_PATH, MISSION_SPLIT_INVOKABLE, 2), MapHit.Waypoint(2), PlanLayer.Mission))
    }
}
