package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class PlanFirstActionTest {

    private fun emptyPlanText(canAddByHand: Boolean, canPlaceByButton: Boolean) = planSummary(
        itemCount = 0,
        shape = emptyList(),
        items = emptyList(),
        polygons = emptyList(),
        circles = emptyList(),
        rally = emptyList(),
        surveys = emptyList(),
        summaryText = "",
        selected = null,
        offline = true,
        canAddByHand = canAddByHand,
        canPlaceByButton = canPlaceByButton,
    )

    @Test
    fun `with a takeoff required and nowhere to put it, neither affordance works`() {
        assertEquals(
            "measured on a fresh Plan with no vehicle: the Takeoff button refuses with 'Move the " +
                "map to where this should go' because a default MapLibre camera sits on (0,0), " +
                "which isPlottable rejects as the no-position sentinel; and a long press inserts " +
                "a WAYPOINT, which the takeoff-first rule refuses. Telling the operator to add a " +
                "takeoff names the one thing that cannot work",
            "Empty plan · move the map to where you will fly, then add a takeoff",
            emptyPlanText(canAddByHand = false, canPlaceByButton = false),
        )
    }

    @Test
    fun `once the map is somewhere real, the takeoff button is the instruction again`() {
        assertEquals(
            "Empty plan · add a takeoff to start",
            emptyPlanText(canAddByHand = false, canPlaceByButton = true),
        )
    }

    @Test
    fun `where a waypoint may be added by hand the long press stays the instruction`() {
        assertEquals(
            "long press carries its own position, so it works wherever the camera is",
            "Empty plan · long press to add",
            emptyPlanText(canAddByHand = true, canPlaceByButton = false),
        )
    }
}
