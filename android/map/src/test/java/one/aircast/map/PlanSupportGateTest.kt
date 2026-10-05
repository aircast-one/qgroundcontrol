package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PlanSupportGateTest {

    private fun plan(addFence: Boolean, addRally: Boolean) = JSONObject(
        """{"actions":{"addFence":$addFence,"addRally":$addRally}}""",
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
    fun `absent actions refuse rather than assume`() {
        val nothing = planSupport(JSONObject("""{}"""))
        assertFalse(nothing.fence)
        assertFalse(nothing.rally)
    }
}
