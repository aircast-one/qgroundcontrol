package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FleetActionsTest {

    private fun served(vararg entries: String) = JSONObject(
        """{"kind":"object","class":"Vehicles","actions":[${entries.joinToString(",")}]}""",
    )

    private fun action(id: String, offer: String, reason: String = "") =
        """{"id":"$id","title":"$id","prompt":"do $id","offer":"$offer","reason":"$reason"}"""

    @Test
    fun `an absent actions list reads as no fleet actions`() {
        assertEquals(emptyList<MvAction>(), mvActions(JSONObject("""{"kind":"object"}""")))
        assertEquals(emptyList<MvAction>(), mvActions(null))
    }

    @Test
    fun `every served action is offered in the order the core listed them`() {
        val actions = mvActions(
            served(
                action("mvArm", "ready"),
                action("mvDisarm", "blocked", "No selected vehicle is armed."),
            ),
        )
        assertEquals(listOf("mvArm", "mvDisarm"), actions.map { it.id })
        assertTrue(actions.first().ready)
        assertFalse(actions.last().ready)
    }

    @Test
    fun `an entry without an id is not offered`() {
        assertEquals(
            emptyList<MvAction>(),
            mvActions(served("""{"title":"Arm","offer":"ready","prompt":"","reason":""}""")),
        )
    }

    @Test
    fun `a reason is shown only while the action is blocked`() {
        val blocked = mvActions(served(action("mvPause", "blocked", "No selected vehicle supports being paused."))).single()
        assertEquals("No selected vehicle supports being paused.", mvReasonFor(blocked))
        val ready = mvActions(served(action("mvPause", "ready", "No selected vehicle supports being paused."))).single()
        assertNull(mvReasonFor(ready))
    }

    @Test
    fun `an empty reason on a blocked action says nothing rather than an empty line`() {
        assertNull(mvReasonFor(mvActions(served(action("mvArm", "blocked"))).single()))
    }

    @Test
    fun `the heading says what the block commands, not just how many are ticked`() {
        assertEquals("Select aircraft to command together", fleetHeading(0))
        assertEquals("Command 1 aircraft together", fleetHeading(1))
        assertEquals("Command 3 aircraft together", fleetHeading(3))
    }

    @Test
    fun `a ready action says what it does`() {
        val ready = mvActions(served(action("mvArm", "ready"))).single()
        assertEquals("do mvArm", fleetActionLine(ready))
    }

    @Test
    fun `a blocked action shows the refusal in place of the prompt`() {
        val blocked = mvActions(served(action("mvDisarm", "blocked", "No selected vehicle is armed."))).single()
        assertEquals("No selected vehicle is armed.", fleetActionLine(blocked))
    }

    @Test
    fun `an action the core sent no prompt for falls back to its title`() {
        assertEquals("Arm", fleetActionLine(MvAction(id = "mvArm", title = "Arm", prompt = "", offer = "ready", reason = "")))
    }

    @Test
    fun `the confirm carries the count, which is what the row does not say`() {
        assertEquals("This commands 1 aircraft.", fleetConfirm(1))
        assertEquals("This commands 4 aircraft.", fleetConfirm(4))
    }

    @Test
    fun `pause is the one fleet action that is not destructive`() {
        val destructive = listOf("mvArm", "mvDisarm", "mvStartMission", "mvPause")
            .map { MvAction(id = it, title = it, prompt = "", offer = "ready", reason = "") }
            .filter(::fleetIsDestructive)
            .map { it.id }
        assertEquals(listOf("mvArm", "mvDisarm", "mvStartMission"), destructive)
    }

    @Test
    fun `the active vehicle's id comes off the view that already names it`() {
        assertEquals(7, activeVehicleId(JSONObject("""{"kind":"object","activeId":7}""")))
    }

    @Test
    fun `no vehicle is no id, and never zero`() {
        assertNull(activeVehicleId(null))
        assertNull(activeVehicleId(JSONObject("""{"kind":"object","activeId":null}""")))
        assertNull(activeVehicleId(JSONObject("""{"kind":"object"}""")))
        assertNull(activeVehicleId(JSONObject("""{"kind":"object","activeId":0}""")))
    }
}
