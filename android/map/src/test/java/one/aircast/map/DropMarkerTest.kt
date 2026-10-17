package one.aircast.map

import org.junit.Assert.assertEquals
import org.junit.Test

class DropMarkerTest {
    private fun waypoint(index: Int) = MissionItem(index, index, 47.0 + index, 8.0, "Waypoint", false, commandId = 16, specifiesCoordinate = true)

    private fun drop(index: Int) = MissionItem(index, index, Double.NaN, Double.NaN, "Gripper Action", false, commandId = DROP_COMMAND, placed = false)

    @Test
    fun `a waypoint followed by a gripper release is marked as the drop point`() {
        val items = listOf(waypoint(1), drop(2), waypoint(3), MissionItem(4, 4, Double.NaN, Double.NaN, "Change Speed", false, commandId = 178, placed = false), waypoint(5))
        assertEquals(listOf(true, false, false, false, false), items.map { dropsAt(items, it) })
        val sides = missionFeatures(items).features().orEmpty().associate { it.getNumberProperty(WAYPOINT_ID_PROPERTY).toInt() to it.getStringProperty(WAYPOINT_SIDE_LABEL_PROPERTY) }
        assertEquals(DROP_SIDE, sides[1])
        assertEquals("", sides[3])
    }
}
