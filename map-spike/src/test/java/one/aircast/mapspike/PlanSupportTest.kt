package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PlanSupportTest {

    private fun view(json: String) = planSupport(JSONObject(json))

    @Test
    fun `a vehicle that takes both offers both`() {
        val both = view("""{"kind":"object","actions":{"addFence":true,"addRally":true},"unsupportedReason":""}""")

        assertTrue(both.fence)
        assertTrue(both.rally)
        assertEquals("", both.reason)
    }

    @Test
    fun `a vehicle that takes neither says so instead of offering buttons that fail`() {
        val neither = view(
            """{"kind":"object","actions":{"addFence":false,"addRally":false},
               "unsupportedReason":"This vehicle accepts neither a geofence nor rally points."}""",
        )

        assertFalse(neither.fence)
        assertFalse(neither.rally)
        assertTrue(neither.reason.contains("neither"))
    }

    @Test
    fun `the two are answered separately, because a vehicle can take one and not the other`() {
        val fenceOnly = view("""{"kind":"object","actions":{"addFence":true,"addRally":false}}""")

        assertTrue(fenceOnly.fence)
        assertFalse(fenceOnly.rally)
    }

    @Test
    fun `no answer hides the buttons rather than offering an action the vehicle will refuse`() {
        assertFalse(planSupport(null).fence)
        assertFalse(planSupport(null).rally)
        assertFalse(view("""{"kind":"null"}""").fence)
    }
}
