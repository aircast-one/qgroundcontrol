package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ConfirmTrackTest {

    @Test
    fun `sent stands until the served actions change`() {
        assertTrue(
            "the operator needs to see the command left the handset while the link is still " +
                "carrying it; reverting to buttons straight away reads as nothing happened",
            sentIsStillShowing("Land", "{A}", "{A}"),
        )
        assertFalse(
            "once the vehicle's own flags move, the next state answers the question and the " +
                "notice has nothing left to say",
            sentIsStillShowing("Land", "{A}", "{B}"),
        )
    }

    @Test
    fun `nothing was sent means nothing is shown`() {
        assertFalse(sentIsStillShowing(null, "{A}", "{A}"))
        assertFalse(sentIsStillShowing("Land", null, "{A}"))
    }

    @Test
    fun `a send recorded against no reading does not stick`() {
        assertFalse(
            "a null snapshot at send would match a null live read and hold the notice forever",
            sentIsStillShowing("Land", null, null),
        )
    }

    @Test
    fun `the notice names the action that was sent`() {
        assertEquals("Sent · Return", sentText("Return"))
        assertEquals("Sent · Emergency Stop", sentText("Emergency Stop"))
    }
}
