package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

private fun number(
    value: String,
    longDescription: String = "",
    min: String = "",
    max: String = "",
    bounded: Boolean = false,
    isString: Boolean = false,
) = Fact(
    path = "vehicle.parameterManager.getParameter(-1,GF_MAX_VER_DIST)",
    name = "GF_MAX_VER_DIST",
    description = "Max vertical distance from Home",
    units = "m",
    valueString = value,
    value = value.toDoubleOrNull(),
    enumStrings = emptyList(),
    enumIndex = -1,
    isBool = false,
    isString = isString,
    readOnly = false,
    minString = min,
    maxString = max,
    minIsDefaultForType = !bounded,
    maxIsDefaultForType = !bounded,
    longDescription = longDescription,
)

class ValueRowTest {
    @Test
    fun `a limit that zero disables reads Off at zero and its value otherwise`() {
        assertEquals(VALUE_OFF, valueText(number("0", "Disabled if 0.")))
        assertEquals("120 m", valueText(number("120", "Disabled if 0.")))
        assertEquals("0 m", valueText(number("0", "Height above home.")))
    }

    @Test
    fun `steps follow the value's own precision and stay inside its range`() {
        assertEquals(0.1, valueStep(number("12.0")), 1e-9)
        assertEquals(1.0, valueStep(number("30")), 1e-9)
        assertEquals(20.0, steppedValue(number("20", min = "0", max = "20", bounded = true), 1)!!, 1e-9)
        assertEquals(29.0, steppedValue(number("30"), -1)!!, 1e-9)
        assertEquals(0.8, steppedValue(number("0.7"), 1)!!, 1e-9)
        assertEquals(3456.0, roundedValue(number("30"), 3456.237), 1e-9)
    }

    @Test
    fun `a slider only appears for a bounded, sensible range`() {
        assertEquals(10000f, derivedSlider(number("0", min = "0", max = "10000", bounded = true))?.to)
        assertNull(derivedSlider(number("30")))
        assertNull(derivedSlider(number("30", min = "0", max = "340282346638528859811704183484516925440", bounded = true)))
    }

    @Test
    fun `numbers open as a value row and text stays a field`() {
        assertTrue(opensAsValue(number("30")))
        assertFalse(opensAsValue(number("127.0.0.1", isString = true)))
    }
}
