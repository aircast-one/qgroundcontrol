package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertNull
import org.junit.Assert.assertEquals
import org.junit.Test

class SetupScreenTest {
    @Test
    fun `the firmware line is the one the core spelled`() {
        val line = firmwareLine(
            JSONObject("""{"kind":"object","available":true,"summary":"PX4 Pro 1.15.0 beta","vehicleType":"Quadrotor"}"""),
        )
        assertEquals(FirmwareLine("PX4 Pro 1.15.0 beta", "Quadrotor"), line)
    }

    @Test
    fun `no vehicle reads as empty rather than stray separators`() {
        assertEquals(FirmwareLine("", ""), firmwareLine(null))
        assertEquals(FirmwareLine("", ""), firmwareLine(JSONObject("""{"kind":"object","available":false,"summary":"","vehicleType":""}""")))
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

    @Test
    fun `a parameter load that has stopped is not drawn as one still running`() {
        val view = { reason: String, ready: Boolean ->
            org.json.JSONObject(
                """{"parametersReady":$ready,"parametersReason":"$reason",
                    "parametersText":"This vehicle has not answered the request for its parameters, and the retries are finished."}""",
            )
        }

        assertEquals("Loading parameters from the vehicle.", parameterWait(view("loading", false))?.title)
        assertEquals("", parameterWait(view("loading", false))?.body)

        val stopped = parameterWait(view("unanswered", false))
        assertEquals(
            "the core states what the vehicle did",
            "This vehicle has not answered the request for its parameters, and the retries are finished.",
            stopped?.title,
        )
        assertEquals(
            "and this head owns the only thing an operator can act on from here",
            "Setup needs them. Disconnect and connect the link to ask again.",
            stopped?.body,
        )

        assertNull("a ready vehicle waits for nothing", parameterWait(view("", true)))
        assertNull(parameterWait(view("noVehicle", false)))
        assertNull("the screen already says to connect a vehicle", parameterWait(null))
    }

    @Test
    fun `a reason this head has never heard of reads as stopped, not as ready`() {
        val future = JSONObject(
            """{"parametersReady":false,"parametersReason":"refused","parametersText":"The vehicle refused the request."}""",
        )
        val waiting = parameterWait(future)
        assertEquals("The vehicle refused the request.", waiting?.title)
        assertEquals(
            "ready would hide a dead load behind a normal screen and loading is the defect being fixed, so an unfamiliar state is closer to stopped than to either",
            PARAMETERS_STOPPED,
            waiting?.body,
        )
    }

    @Test
    fun `a reason with no sentence beside it still says something true`() {
        val bare = JSONObject("""{"parametersReady":false,"parametersReason":"refused","parametersText":""}""")
        assertEquals("This vehicle has not sent its parameters (refused).", parameterWait(bare)?.title)
    }
}
