package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class GuidedStepperTest {
    @Test
    fun `a step moves one unit and stays inside the range, like GuidedValueSlider step`() {
        assertEquals(26.0, guidedStepped(25.0, 1, 1.0, 120.0, "m"), 1e-9)
        assertEquals(120.0, guidedStepped(119.6, 1, 1.0, 120.0, "m"), 1e-9)
        assertEquals(1.0, guidedStepped(1.4, -1, 1.0, 120.0, "m"), 1e-9)
    }

    @Test
    fun `values round to tenths in metric and whole numbers otherwise`() {
        assertEquals(25.4, guidedRounded(25.43, "m"), 1e-9)
        assertEquals(25.0, guidedRounded(25.43, "ft"), 1e-9)
        assertEquals("25.4", guidedValueText(25.4, "m"))
        assertEquals("25", guidedValueText(25.0, "ft"))
    }

    @Test
    fun `a typed value is clamped and rounded, and garbage is ignored`() {
        assertEquals(120.0, guidedTyped("500", 1.0, 120.0, "m")!!, 1e-9)
        assertEquals(12.5, guidedTyped("12,46", 1.0, 120.0, "m")!!, 1e-9)
        assertNull(guidedTyped("x", 1.0, 120.0, "m"))
        assertNull(guidedBounds(null, 10.0))
    }
}
