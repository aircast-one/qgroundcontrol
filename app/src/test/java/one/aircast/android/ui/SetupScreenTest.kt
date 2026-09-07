package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class SetupScreenTest {
    @Test
    fun `a released firmware reads as type and version`() {
        assertEquals("ArduPilot 4.5.7", firmwareSummary("ArduPilot", 4, 5, 7, "Official"))
    }

    @Test
    fun `a prerelease keeps its version type`() {
        assertEquals("PX4 1.15.0 beta", firmwareSummary("PX4", 1, 15, 0, "beta"))
    }

    @Test
    fun `an unknown version falls back to the firmware name`() {
        assertEquals("ArduPilot", firmwareSummary("ArduPilot", -1, 0, 0, ""))
    }

    @Test
    fun `nothing known reads as empty rather than stray separators`() {
        assertEquals("", firmwareSummary("", -1, 0, 0, ""))
    }

    @Test
    fun `a component only needs attention when setup is required and missing`() {
        val required = SetupComponent(0, "Radio", "", requiresSetup = true, setupComplete = false)
        val done = SetupComponent(1, "Radio", "", requiresSetup = true, setupComplete = true)
        val optional = SetupComponent(2, "Camera", "", requiresSetup = false, setupComplete = false)
        assertEquals(true, required.needsAttention)
        assertEquals(false, done.needsAttention)
        assertEquals(false, optional.needsAttention)
    }
}
