package one.aircast.android.ui

import org.junit.Assert.assertEquals
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
