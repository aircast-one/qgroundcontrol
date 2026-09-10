package one.aircast.android.ui

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
