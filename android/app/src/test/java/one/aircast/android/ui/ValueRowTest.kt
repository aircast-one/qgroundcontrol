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
    name: String = "MPC_XY_VEL_MAX",
    default: String = "",
    units: String = "m",
) = Fact(
    path = "vehicle.parameterManager.getParameter(-1,GF_MAX_VER_DIST)",
    name = name,
    description = "Max vertical distance from Home",
    units = units,
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
    defaultValueString = default,
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

    @Test
    fun `turning a limit on lands on a safe value, never one step above zero`() {
        assertEquals(120.0, turnOnValue(number("0", "Disabled if 0.", default = "0", name = "GF_MAX_VER_DIST")), 1e-9)
        assertEquals(500.0, turnOnValue(number("0", "Disabled if 0.", name = "GF_MAX_HOR_DIST")), 1e-9)
        assertEquals(12000.0, turnOnValue(number("0", "Disabled if 0.", name = "FENCE_ALT_MAX", units = "cm")), 1e-9)
        assertEquals(80.0, turnOnValue(number("0", "Disabled if 0.", name = "OTHER", default = "80")), 1e-9)
        assertEquals(120.0, nudged(number("0", "Disabled if 0.", name = "GF_MAX_VER_DIST"), 0.0, 1, 0), 1e-9)
    }

    @Test
    fun `altitudes and distances step in whole metres whatever their precision`() {
        assertEquals(1.0, valueStep(number("30.0", name = "RTL_RETURN_ALT")), 1e-9)
        assertEquals(100.0, valueStep(number("3000", name = "RTL_ALT", units = "cm")), 1e-9)
    }

    @Test
    fun `holding a step button speeds up after a while`() {
        assertEquals(31.0, nudged(number("30"), 30.0, 1, 3), 1e-9)
        assertEquals(40.0, nudged(number("30"), 30.0, 1, 12), 1e-9)
    }

    @Test
    fun `return and fence sliders cover the useful band, not the whole type range`() {
        val slider = sliderFor(number("30", name = "RTL_RETURN_ALT"))
        assertEquals(20f, slider?.from)
        assertEquals(500f, slider?.to)
        assertEquals(50f, sliderFor(number("0", name = "GF_MAX_HOR_DIST", min = "0", max = "10000", bounded = true))?.from)
    }

    @Test
    fun `a pending value reads like the row`() {
        assertEquals(VALUE_OFF, valueTextOf(number("120", "Disabled if 0."), 0.0))
        assertEquals("45.5 m", valueTextOf(number("30.0"), 45.5))
    }

    @Test
    fun `a long choice label is cut at its first clause in the row`() {
        assertEquals("Return at critical level", rowChoiceLabel("Return at critical level, land at emergency level"))
        assertEquals("Hold mode", rowChoiceLabel("Hold mode"))
    }
}
