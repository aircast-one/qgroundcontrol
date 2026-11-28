package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

class ActionOrderTest {
    private val anywhere = "[0,1,2,3]"

    private fun steps(hoverPlaces: String = anywhere, otherPlaces: String = anywhere): List<ActionStep> = actionSteps(
        JSONObject(
            """{"actions":[
            {"kind":"Hover","title":"Hover","seconds":10,"item":null,"places":$hoverPlaces,"refusal":"stays first"},
            {"kind":"Gimbal","title":"Gimbal","seconds":null,"item":null,"places":$otherPlaces,"refusal":null},
            {"kind":"Camera","title":"Camera","seconds":null,"item":null,"places":$otherPlaces,"refusal":null},
            {"kind":"Command","title":"Set servo","seconds":null,"item":5,"places":$otherPlaces,"refusal":null}]}""",
        ),
    )

    private val active = listOf(
        WaypointAction(ActionKind.Hover, "Hover", "10 s", R.drawable.plan_timer),
        WaypointAction(ActionKind.Gimbal, "Gimbal", "0° / 60°", R.drawable.plan_gimbal),
        WaypointAction(ActionKind.Camera, "Take photo", "", R.drawable.plan_photo),
        WaypointAction(ActionKind.Turn, "Turn aircraft", "90°", R.drawable.plan_turn),
    )

    @Test
    fun `the rows read the core's order and turning the aircraft stays last with nowhere to drag`() {
        val ordered = orderedActions(steps(), active)
        assertEquals(listOf("Hover", "Gimbal", "Take photo", "Set servo", "Turn aircraft"), ordered.map { it.title })
        assertEquals(listOf(0, 1, 2, 3, null), ordered.map { it.place })
        assertFalse("turning the aircraft is set at the waypoint and is never dragged", canDrop(ordered.last(), 0))
        assertEquals(listOf(0, 1, 2, 3), steps().first().places)
        assertEquals(emptyList<ActionStep>(), actionSteps(null))
    }

    @Test
    fun `a slot the aircraft cannot fly opens nothing and letting go there sends nothing`() {
        val ordered = orderedActions(steps("[0]", "[1,2,3]"), active).filter { it.place != null }
        assertEquals("held over the hold, which may not move, no row makes room", listOf(0, 1, 2, 3), projectedPlaces(ordered, 3, 0))
        assertNull(dropMove(2, ordered[3], 0))
        assertEquals(listOf(0, 2, 3, 1), projectedPlaces(ordered, 3, 1))
        assertEquals("[2, 3, 1]", dropMove(2, ordered[3], 1))
        assertNull("let go where it started", dropMove(2, ordered[3], 3))
        assertNull("let go outside the list", dropMove(2, ordered[3], null))
    }

    @Test
    fun `the hold reads as a wait as soon as it would leave first place`() {
        val ordered = orderedActions(steps(), active).filter { it.place != null }
        val hold = ordered.first()
        assertEquals(listOf("Hover" to R.drawable.plan_timer, "Wait" to R.drawable.plan_pause), listOf(0, 2).map { shownAt(hold, it) }.map { it.title to it.icon })
        val projected = projectedPlaces(ordered, 2, 0)
        assertEquals("the photo dragged to the top pushes the hold to second", listOf(1, 2, 0, 3), projected)
        assertEquals(WAIT, shownAt(hold, projected[0]).title)
        assertEquals("Gimbal", shownAt(ordered[1], 3).title)
    }

    @Test
    fun `a drag through every slot sends one move, for the place it was let go`() {
        val ordered = orderedActions(steps(), active).filter { it.place != null }
        val centres = listOf(20f, 70f, 120f, 170f)
        val path = listOf(-30f, -30f, -30f, -30f, -30f)
        val dragged = path.runningFold(ReorderDrag(3)) { drag, dy -> drag.movedBy(dy, centres, 0f, 190f, 32f) }
        assertEquals("passing each row only moves the slot", listOf(3, 2, 2, 1, 1, 0), dragged.map { it.over })
        assertEquals("only letting go sends anything, and it is one move from where it was to where it lands", "[2, 3, 0]", dragged.last().released(2, ordered))
        assertNull("dragged off the list", ReorderDrag(3).movedBy(400f, centres, 0f, 190f, 32f).released(2, ordered))
        assertEquals(listOf(20, 1, 10), movedOrder(listOf(10, 20, 1), 0, 2))
    }
}

class StaleRowsTest {
    @Test
    fun `rows read before the plan last changed are not acted on`() {
        assertEquals(5L, planRevision(JSONObject("""{"revision":5}""")))
        assertEquals(NO_REVISION, planRevision(null))
        assertEquals(listOf(true, false, false), listOf(6L to 5L, 5L to 5L, 4L to 5L).map { (plan, rows) -> rowsAreStale(plan, rows) })
    }
}
