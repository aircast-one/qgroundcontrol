package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PlanItemsTest {
    private fun item(
        index: Int,
        sequence: Int,
        name: String,
        altitude: Double = 50.0,
        kind: String = "waypoint",
        commandId: Int = 16,
    ) = MissionItem(
        index, sequence, 41.0, 44.0, name, false, altitude,
        kind = kind, commandId = commandId,
    )

    @Test
    fun `a row carries the number, the name, the altitude and the marker colour`() {
        val rows = itemRows(listOf(item(1, 1, "Waypoint", altitude = 49.6)))

        assertEquals(listOf("1"), rows.map { it.number })
        assertEquals(listOf("Waypoint"), rows.map { it.name })
        assertEquals(listOf("50 m"), rows.map { it.detail })
        assertEquals(listOf(WAYPOINT_COLOUR), rows.map { it.colour })
    }

    @Test
    fun `the list is coloured the same way the map is, so a row and its marker match`() {
        val rows = itemRows(
            listOf(
                item(0, 0, "Mission Start", kind = "settings"),
                item(1, 1, "Takeoff", kind = "takeoff", commandId = 22),
                item(2, 2, "Return To Launch", kind = "command", commandId = MAV_CMD_NAV_RETURN_TO_LAUNCH),
            ),
        )

        assertEquals(listOf(START_COLOUR, TAKEOFF_COLOUR, RETURN_COLOUR), rows.map { it.colour })
    }

    @Test
    fun `an item with no altitude says nothing rather than NaN`() {
        assertEquals("", itemRows(listOf(item(1, 1, "Waypoint", altitude = Double.NaN))).single().detail)
    }

    @Test
    fun `an item the vehicle did not name is still reachable by its number`() {
        assertEquals("Item 3", itemRows(listOf(item(3, 3, ""))).single().name)
    }

    @Test
    fun `an item the map cannot draw is still listed, and says why it cannot be tapped`() {
        val rows = itemRows(
            listOf(
                MissionItem(
                    1, 1, Double.NaN, Double.NaN, "Change Speed", false, Double.NaN,
                    kind = "command", commandId = 178, placed = false,
                ),
            ),
        )

        assertEquals(NO_POSITION, rows.single().detail)
        assertEquals(false, rows.single().placed)
    }

    @Test
    fun `a row is found by the index the map selects with, not by position`() {
        val rows = itemRows(listOf(item(4, 1, "Waypoint"), item(9, 2, "Land", kind = "land", commandId = 21)))

        assertEquals("Land", rowAt(rows, 9)?.name)
        assertNull(rowAt(rows, 1))
    }
}
