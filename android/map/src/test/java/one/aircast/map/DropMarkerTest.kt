package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class DropMarkerTest {
    private fun waypoint(index: Int) = MissionItem(index, index, 47.0 + index, 8.0, "Waypoint", false, commandId = 16, specifiesCoordinate = true)

    private fun drop(index: Int) = MissionItem(index, index, Double.NaN, Double.NaN, "Gripper Action", false, commandId = 211, dropsPayload = true, placed = false)

    @Test
    fun `a waypoint followed by a gripper release is marked as the drop point`() {
        val items = listOf(waypoint(1), drop(2), waypoint(3), MissionItem(4, 4, Double.NaN, Double.NaN, "Change Speed", false, commandId = 178, placed = false), waypoint(5))
        assertEquals(listOf(true, false, false, false, false), items.map { dropsAt(items, it) })
        val sides = missionFeatures(withDrops(items).filter { it.placed }).features().orEmpty().associate { it.getNumberProperty(WAYPOINT_ID_PROPERTY).toInt() to it.getStringProperty(WAYPOINT_SIDE_LABEL_PROPERTY) }
        assertEquals(DROP_SIDE, sides[1])
        assertEquals("", sides[3])
    }

    @Test
    fun `a bay servo drop marks its waypoint, and a servo that drops nothing does not`() {
        fun servo(index: Int, drops: Boolean) = MissionItem(index, index, Double.NaN, Double.NaN, "Set Servo", false, commandId = 183, dropsPayload = drops, placed = false)
        val items = listOf(waypoint(1), servo(2, true), waypoint(3), servo(4, false), waypoint(5))
        assertEquals(listOf(true, false, false, false, false), items.map { dropsAt(items, it) })
    }

    @Test
    fun `the plan map read marks the waypoint before a drop, though the drop itself has no place`() {
        val plan = allMissionItems(
            JSONObject(
                """{"kind":"object","items":[
                     {"sequence":1,"name":"Waypoint","command":16,"specifiesCoordinate":true,"coordinate":{"latitude":47.0,"longitude":8.0}},
                     {"sequence":2,"name":"Drop payload","command":211,"dropsPayload":true}]}""",
            ),
        )

        assertEquals(listOf(true), plan.filter { it.placed }.map { it.drops })
    }
}
