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
}
