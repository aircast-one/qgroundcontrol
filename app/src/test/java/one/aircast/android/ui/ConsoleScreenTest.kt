package one.aircast.android.ui

import org.junit.Assert.assertEquals
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
