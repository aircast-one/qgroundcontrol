package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PlanSupportGateTest {

    private fun plan(addFence: Boolean, addRally: Boolean, reason: String = "") = JSONObject(
        """{"actions":{"addFence":$addFence,"addRally":$addRally},"unsupportedReason":"$reason"}""",
    )

    @Test
    fun `a firmware without fences refuses both fence shapes, not just the polygon`() {
        val support = planSupport(plan(addFence = false, addRally = true))
        assertFalse(
            "the Fence button adds an inclusion POLYGON and the Circle button an inclusion " +
                "CIRCLE - both are geofences and both must answer to addFence. Circle carried no " +
                "enabled gate at all and stayed live on a firmware that refuses fences",
            support.fence,
        )
        assertTrue("rally is a separate capability and is unaffected", support.rally)
    }

    @Test
    fun `the reason travels so both buttons can say the same thing`() {
        assertEquals(
            "the polygon button already reported this on completion; the circle reported nothing",
            "This firmware has no geofence support.",
            planSupport(plan(false, false, "This firmware has no geofence support.")).reason,
        )
    }

    @Test
    fun `absent actions refuse rather than assume`() {
        val nothing = planSupport(JSONObject("""{}"""))
        assertFalse(nothing.fence)
        assertFalse(nothing.rally)
    }
}
