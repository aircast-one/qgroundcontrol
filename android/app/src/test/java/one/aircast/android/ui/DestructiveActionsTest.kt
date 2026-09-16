package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DestructiveActionsTest {

    private fun served(vararg entries: String) = JSONObject(
        """{"kind":"object","class":"Camera","present":true,
            "destructiveActions":[${entries.joinToString(",")}]}""",
    )

    private fun action(id: String, offer: String, reason: String = "", prompt: String = "P") =
        """{"id":"$id","title":"$id","prompt":"$prompt","offer":"$offer",
            "reason":"$reason","destructive":true}"""

    @Test
    fun `a hidden action is not drawn at all`() {
        val shown = destructiveActions(
            served(action("formatStorage", "hidden"), action("resetSettings", "ready")),
        )

        assertEquals(listOf("resetSettings"), shown.map { it.id })
    }

    @Test
    fun `a blocked action is drawn, disabled, with the reason the core gave`() {
        val blocked = destructiveActions(
            served(action("formatStorage", "blocked", reason = "The camera is recording.")),
        ).single()

        assertFalse("the row stays, so the operator sees why rather than why not", blocked.ready)
        assertTrue(blocked.blocked)
        assertEquals("The camera is recording.", destructiveReasonFor(blocked))
    }

    @Test
    fun `a ready action carries no reason to show`() {
        val ready = destructiveActions(served(action("resetSettings", "ready"))).single()

        assertTrue(ready.ready)
        assertNull(
            "a reason beneath an enabled button reads as a warning about the action rather " +
                "than an explanation of why it cannot be used",
            destructiveReasonFor(ready),
        )
    }

    @Test
    fun `the confirmation body is the core's sentence`() {
        val reset = destructiveActions(
            served(
                action(
                    "resetSettings",
                    "ready",
                    prompt = "Put every camera setting back to its factory value. This cannot be undone.",
                ),
            ),
        ).single()

        assertEquals(
            "the head used to carry Qt's literal question here; two sources of truth for one " +
                "destructive action had already disagreed on wording",
            "Put every camera setting back to its factory value. This cannot be undone.",
            reset.prompt,
        )
    }

    @Test
    fun `an action this head cannot send is not given a path`() {
        assertEquals(CAMERA_RESET, destructiveInvokePath("resetSettings"))
        assertEquals(CAMERA_FORMAT, destructiveInvokePath("formatStorage"))
        assertNull(destructiveInvokePath("somethingLater"))
    }

    @Test
    fun `a core too old to serve them leaves the sheet without the rows`() {
        assertEquals(emptyList<DestructiveAction>(), destructiveActions(null))
        assertEquals(
            emptyList<DestructiveAction>(),
            destructiveActions(JSONObject("""{"kind":"object","class":"Camera"}""")),
        )
    }
}
