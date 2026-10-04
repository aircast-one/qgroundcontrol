package one.aircast.android.ui

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ConsoleScreenTest {
    @Test
    fun `an empty console shows the bare prompt like the MAVLinkConsolePage text area`() {
        assertEquals("> ", CONSOLE_EMPTY_TEXT)
    }

    @Test
    fun `opening the page asks the core for a fresh controller`() {
        assertEquals("mavlinkConsole.open", CONSOLE_OPEN)
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

class ConsoleLinesTest {
    @Test
    fun `lines come from the served array`() {
        assertEquals(listOf("a", "b"), consoleLines(JSONObject("""{"lines":["a","b"]}""")))
        assertEquals(emptyList<String>(), consoleLines(JSONObject("{}")))
        assertEquals(emptyList<String>(), consoleLines(null))
    }

    @Test
    fun `a leading WARN or ERROR is coloured, as MAVLinkConsoleController marks it`() {
        val warn = androidx.compose.ui.graphics.Color.Yellow
        val error = androidx.compose.ui.graphics.Color.Red
        val warned = consoleLineStyled("WARN  [sensors] no baro", warn, error)
        assertEquals(listOf(Triple(0, 4, warn)), warned.spanStyles.map { Triple(it.start, it.end, it.item.color) })
        assertEquals(listOf(Triple(0, 5, error)), consoleLineStyled("ERROR x", warn, error).spanStyles.map { Triple(it.start, it.end, it.item.color) })
        assertTrue("case sensitive, and only at the start", consoleLineStyled("warn: an INFO WARN", warn, error).spanStyles.isEmpty())
    }

    @Test
    fun promptLinesAreTheEchoedCommands() {
        assertTrue(isPromptLine("nsh> free"))
        org.junit.Assert.assertFalse(isPromptLine("total  used  free"))
        org.junit.Assert.assertFalse(isPromptLine("note: nsh> appears mid-line"))
    }

}

class ConsolePasteTest {
    @Test
    fun `a pasted block sends its complete lines and keeps the unfinished tail like handleClipboard`() {
        val (sent, left) = splitCompleteLines(TextFieldValue("ver all\nfree\nto", TextRange(15)))
        assertEquals("ver all\nfree", sent)
        assertEquals("to", left.text)
        assertEquals(TextRange(2), left.selection)
    }

    @Test
    fun `text after the cursor stays behind the pasted tail`() {
        val (sent, left) = splitCompleteLines(TextFieldValue("lsfree\nps\n -l", TextRange(10)))
        assertEquals("lsfree\nps", sent)
        assertEquals(" -l", left.text)
        assertEquals(TextRange(0), left.selection)
    }

    @Test
    fun `typing without a newline sends nothing`() {
        val field = TextFieldValue("help", TextRange(4))
        assertEquals(null to field, splitCompleteLines(field))
    }
}
