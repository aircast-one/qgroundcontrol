package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class GuidedStepperTest {
    @Test
    fun `the hold names its target and says anyway only for a non-blocking warning`() {
        assertEquals("Take off \u00b7 3.0 m", guidedCommitLabel("Take off \u00b7 3.0 m", null))
        assertEquals("Take off anyway \u00b7 3.0 m", guidedCommitLabel("Take off \u00b7 3.0 m", Readiness("GPS off", blocks = false)))
        assertEquals("Take off \u00b7 3.0 m", guidedCommitLabel("Take off \u00b7 3.0 m", Readiness("GPS off", blocks = true)))
        assertEquals("Hold to take off anyway \u00b7 3.0 m", holdLabel(guidedCommitLabel("Take off \u00b7 3.0 m", Readiness("GPS off", blocks = false))))
    }

    @Test
    fun `takeoff presets are the common heights the vehicle allows`() {
        assertEquals(listOf(5.0, 10.0, 20.0, 50.0), guidedPresets("m", 3.0, 121.9))
        assertEquals(listOf(5.0, 10.0), guidedPresets("m", 3.0, 15.0))
        assertEquals(listOf(15.0, 30.0, 60.0, 150.0), guidedPresets("ft", 10.0, 400.0))
    }

    @Test
    fun `quick picks climb ten and twenty from the target and stop at the maximum`() {
        assertEquals(
            listOf("+10 m" to 52.0, "+20 m" to 62.0, "Max 120 m" to 120.0),
            guidedQuickPicks(42.0, 1.0, 120.0, "m"),
        )
        assertEquals(120.0, guidedQuickPicks(115.0, 1.0, 120.0, "m")[1].second, 1e-9)
    }

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

    @Test
    fun `the reading carries its unit on one line`() {
        assertEquals("10 ft", guidedReading(10.0, "ft"))
        assertEquals("12.5 m", guidedReading(12.5, "m"))
        assertEquals("3", guidedReading(3.0, ""))
    }

    @Test
    fun `the panel names the vehicle only when there is a choice of vehicles`() {
        assertNull(guidedVehicle(1, "Quadrotor 128"))
        assertEquals("Quadrotor 128", guidedVehicle(2, "Quadrotor 128"))
        assertNull(guidedVehicle(2, null))
    }
}
