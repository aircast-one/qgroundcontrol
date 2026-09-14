package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class ConsoleScreenTest {
    @Test
    fun `trailing blank lines are dropped`() {
        assertEquals(
            listOf("nsh>", "help"),
            visibleConsoleLines(listOf("nsh>", "help", "", "  ", "")),
        )
    }

    @Test
    fun `blank lines inside the transcript are kept`() {
        assertEquals(
            listOf("a", "", "b"),
            visibleConsoleLines(listOf("a", "", "b")),
        )
    }

    @Test
    fun `an all blank transcript reads as empty`() {
        assertEquals(emptyList<String>(), visibleConsoleLines(listOf("", " ")))
        assertEquals(emptyList<String>(), visibleConsoleLines(emptyList()))
    }

    @Test
    fun `before anything is sent the empty console says what to try`() {
        assertEquals(
            "No output yet. Send a command, for example help.",
            consoleEmptyText(sent = false, connected = true, servedReason = ""),
        )
    }

    @Test
    fun `after a command goes out the silence is the vehicle's, not the operator's`() {
        assertEquals("Sent. Nothing back from the vehicle yet.", consoleEmptyText(sent = true, connected = true, servedReason = ""))
    }
}

class ConsoleShellHintTest {
    @Test
    fun `a PX4 vehicle gets no hint`() {
        assertNull(consoleShellHint(px4Firmware = true))
    }

    @Test
    fun `another autopilot is warned without being blocked`() {
        val hint = consoleShellHint(px4Firmware = false)

        assertNotNull(hint)
        assertTrue(hint!!.contains("may not reply"))
    }
}

class ConsoleFollowTailTest {
    @Test
    fun `an empty list follows the tail`() {
        assertTrue(shouldFollowTail(null, 0))
    }

    @Test
    fun `a reader at the bottom keeps following`() {
        assertTrue(shouldFollowTail(lastVisibleIndex = 9, count = 10))
    }

    @Test
    fun `a reader one line behind still follows so a new line does not strand them`() {
        assertTrue(shouldFollowTail(lastVisibleIndex = 9, count = 11))
    }

    @Test
    fun `a reader scrolled up is left where they are`() {
        assertFalse(shouldFollowTail(lastVisibleIndex = 3, count = 40))
    }

    @Test
    fun `scrolling up by two lines is enough to stop the yank`() {
        assertFalse(shouldFollowTail(lastVisibleIndex = 7, count = 10))
    }
}

class ConsoleEmptyReasonTest {
    @Test
    fun `with no vehicle the core's sentence is the only correct one`() {
        assertEquals(
            "Connect to a vehicle to open a shell on it.",
            consoleEmptyText(sent = false, connected = false, servedReason = "Connect to a vehicle to open a shell on it."),
        )
    }

    @Test
    fun `once the operator has sent something this screen knows more than the core does`() {
        assertEquals(
            "Sent. Nothing back from the vehicle yet.",
            consoleEmptyText(sent = true, connected = true, servedReason = "The vehicle has printed nothing."),
        )
    }

    @Test
    fun `connected and nothing sent keeps the invitation to type, not the core's flat statement`() {
        assertEquals(
            "No output yet. Send a command, for example help.",
            consoleEmptyText(sent = false, connected = true, servedReason = "The vehicle has printed nothing."),
        )
    }

    @Test
    fun `lines come from the served array`() {
        assertEquals(listOf("a", "b"), consoleLines(JSONObject("""{"lines":["a","b"]}""")))
        assertEquals(emptyList<String>(), consoleLines(JSONObject("{}")))
        assertEquals(emptyList<String>(), consoleLines(null))
    }
}
