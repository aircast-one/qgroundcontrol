package one.aircast.map

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PlanSummaryTest {
    private fun item(index: Int = 0, altitude: Double = Double.NaN) =
        MissionItem(
            index, index + 1, 41.0, 44.0, "Waypoint", false, altitude,
            altitudeText = if (altitude.isNaN()) "" else "${altitude.toInt()} m",
        )

    @Test
    fun `a selected waypoint shows its altitude against the number on its marker`() {
        assertEquals(
            "the map marker and the list row are labelled with the sequence, so naming the " +
                "index here would point at a different item once a survey is in the plan",
            "#4 at 75 m",
            selectionText(MapHit.Waypoint(3), listOf(item(index = 3, altitude = 75.0)), emptyList(), emptyList()),
        )
    }

    @Test
    fun `a selected circle shows its radius from either handle, spelled by the core`() {
        val circles = listOf(FenceCircle(1, true, TrackPoint(41.0, 44.0), 136.0, "446 ft radius"))

        assertEquals("446 ft radius", selectionText(MapHit.Circle(1), emptyList(), circles, emptyList()))
        assertEquals("446 ft radius", selectionText(MapHit.CircleCentre(1), emptyList(), circles, emptyList()))
    }

    @Test
    fun `a selection with nothing to say adds nothing`() {
        assertNull(selectionText(MapHit.Waypoint(0), listOf(item()), emptyList(), emptyList()))
    }
}
