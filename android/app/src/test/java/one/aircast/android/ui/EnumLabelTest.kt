package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Test

private fun fact(value: String, strings: List<String>, index: Int) = Fact(
    path = "p",
    name = "n",
    description = "",
    units = "",
    valueString = value,
    value = value,
    enumStrings = strings,
    enumIndex = index,
    isBool = false,
    isString = false,
    readOnly = true,
)

class EnumLabelTest {
    @Test
    fun `an enum reads as its name, not its number`() {
        assertEquals("RTL", enumLabel(fact("6", listOf("Stabilize", "Land", "RTL"), 2)))
    }

    @Test
    fun `a plain value reads as itself`() {
        assertEquals("42", enumLabel(fact("42", emptyList(), -1)))
    }

    @Test
    fun `an index outside the list falls back to the value`() {
        assertEquals("99", enumLabel(fact("99", listOf("A", "B"), 7)))
        assertEquals("99", enumLabel(fact("99", listOf("A", "B"), -1)))
    }

    @Test
    fun `a settings choice reads in sentence case but an unlisted value stays as typed`() {
        assertEquals("Video stream disabled", shownEnumLabel(fact("0", listOf("Video Stream Disabled", "UDP h.264 Video Stream"), 0)))
        assertEquals("Fit height", shownEnumLabel(fact("1", listOf("Fit Width", "Fit Height"), 1)))
        assertEquals("My Value", shownEnumLabel(fact("My Value", emptyList(), -1)))
    }
}
