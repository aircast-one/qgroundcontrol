package one.aircast.android.ui

import org.json.JSONObject
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
    fun `a component is read with the verdicts the core reached`() {
        val view = JSONObject(
            """{"components":[{"name":"Radio","needsAttention":true,"blockedReason":"armed"},""" +
                """{"name":"Camera","needsAttention":false,"blockedReason":null}]}""",
        )
        val read = setupComponents(view)

        assertEquals(listOf("Radio", "Camera"), read.map { it.name })
        assertEquals(listOf(true, false), read.map { it.needsAttention })
        assertEquals(listOf("armed", null), read.map { it.blockedReason })
    }

    @Test
    fun `a component with no name is not offered, and no view is no components`() {
        assertEquals(1, setupComponents(JSONObject("""{"components":[{"name":"Radio"},{}]}""")).size)
        assertEquals(emptyList<String>(), setupComponents(null).map { it.name })
    }

    @Test
    fun `a component promoted for attention is not listed a second time`() {
        val components = listOf(
            SetupComponent(0, "Frame", needsAttention = false),
            SetupComponent(1, "Sensors", needsAttention = true),
            SetupComponent(2, "Power", needsAttention = false),
        )

        assertEquals(listOf("Frame", "Power"), remainingSetup(components).map { it.name })
    }

    @Test
    fun `every component needing attention leaves nothing for the full list`() {
        val components = listOf(
            SetupComponent(0, "Sensors", needsAttention = true),
        )

        assertEquals(emptyList<String>(), remainingSetup(components).map { it.name })
    }

}
