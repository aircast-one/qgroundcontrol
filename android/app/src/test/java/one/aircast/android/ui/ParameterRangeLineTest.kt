package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Test

private fun fact(min: String = "", max: String = "", default: String = "", bounded: Boolean = true, enum: Boolean = false) = Fact(
    path = "p",
    name = "RTL_ALT",
    description = "Return altitude",
    units = "m",
    valueString = "40",
    value = 40,
    enumStrings = if (enum) listOf("Off", "On") else emptyList(),
    enumIndex = -1,
    isBool = false,
    isString = false,
    readOnly = false,
    minString = min,
    maxString = max,
    minIsDefaultForType = !bounded,
    maxIsDefaultForType = !bounded,
    defaultValueString = default,
)

class ParameterRangeLineTest {
    @Test
    fun `a bounded parameter with a default reads like the Penpot edit sheet`() {
        assertEquals("Range 2–300 m · default 15 m", parameterRangeLine(fact("2", "300", "15")))
    }

    @Test
    fun `type limits and absent defaults say nothing`() {
        assertEquals("", parameterRangeLine(fact("-3.4e38", "3.4e38", bounded = false)))
    }

    @Test
    fun `an enum shows its range and default without units, as ParameterEditorDialog does`() {
        assertEquals("Range 0–1 · default 0", parameterRangeLine(fact("0", "1", "0", enum = true)))
    }

    @Test
    fun `a lone bound is shown on its own`() {
        assertEquals("Min 2 m", parameterRangeLine(fact("2", "").copy(maxIsDefaultForType = true)))
        assertEquals("Max 300 m", parameterRangeLine(fact("", "300").copy(minIsDefaultForType = true)))
    }

    @Test
    fun `reboot notes follow the fact like ParameterEditorDialog`() {
        assertEquals(listOf("Vehicle reboot required after change"), parameterRebootNotes(fact().copy(vehicleRebootRequired = true)))
        assertEquals(emptyList<String>(), parameterRebootNotes(fact()))
    }
}
