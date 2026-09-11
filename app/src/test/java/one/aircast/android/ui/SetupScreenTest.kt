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
        val required = SetupComponent(0, "Radio", requiresSetup = true, setupComplete = false)
        val done = SetupComponent(1, "Radio", requiresSetup = true, setupComplete = true)
        val optional = SetupComponent(2, "Camera", requiresSetup = false, setupComplete = false)
        assertEquals(true, required.needsAttention)
        assertEquals(false, done.needsAttention)
        assertEquals(false, optional.needsAttention)
    }

    @Test
    fun `a component promoted for attention is not listed a second time`() {
        val components = listOf(
            SetupComponent(0, "Frame", requiresSetup = false, setupComplete = false),
            SetupComponent(1, "Sensors", requiresSetup = true, setupComplete = false),
            SetupComponent(2, "Power", requiresSetup = true, setupComplete = true),
        )

        assertEquals(listOf("Frame", "Power"), remainingSetup(components).map { it.name })
    }

    @Test
    fun `every component needing attention leaves nothing for the full list`() {
        val components = listOf(
            SetupComponent(0, "Sensors", requiresSetup = true, setupComplete = false),
        )

        assertEquals(emptyList<String>(), remainingSetup(components).map { it.name })
    }

    private fun component(
        allowArmed: Boolean = false,
        allowFlying: Boolean = false,
    ) = SetupComponent(
        index = 0,
        name = "Sensors",
        requiresSetup = true,
        setupComplete = false,
        allowSetupWhileArmed = allowArmed,
        allowSetupWhileFlying = allowFlying,
    )

    @Test
    fun `a disarmed vehicle on the ground blocks nothing`() {
        assertEquals(null, setupBlockedReason(component(), armed = false, flying = false, isRover = false))
    }

    @Test
    fun `calibration is refused on an armed vehicle`() {
        assertEquals("armed", setupBlockedReason(component(), armed = true, flying = false, isRover = false))
    }

    @Test
    fun `calibration is refused in flight`() {
        assertEquals("flying", setupBlockedReason(component(), armed = false, flying = true, isRover = false))
    }

    @Test
    fun `armed is reported before flying when both are true`() {
        assertEquals("armed", setupBlockedReason(component(), armed = true, flying = true, isRover = false))
    }

    @Test
    fun `a component that permits it is allowed while armed`() {
        assertEquals(
            null,
            setupBlockedReason(component(allowArmed = true), armed = true, flying = false, isRover = false),
        )
    }

    @Test
    fun `a rover is never blocked for flying, matching the desktop`() {
        assertEquals(
            null,
            setupBlockedReason(component(), armed = false, flying = true, isRover = true),
        )
    }

    @Test
    fun `a rover is still blocked while armed`() {
        assertEquals(
            "armed",
            setupBlockedReason(component(), armed = true, flying = false, isRover = true),
        )
    }
}
