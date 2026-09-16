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
    fun `the heading counts the selection`() {
        assertEquals("No vehicles selected", fleetHeading(0))
        assertEquals("1 vehicle selected", fleetHeading(1))
        assertEquals("3 vehicles selected", fleetHeading(3))
    }
}
