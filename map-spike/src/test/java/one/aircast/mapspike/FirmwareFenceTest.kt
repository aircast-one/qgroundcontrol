package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class FirmwareFenceTest {
    private fun view(body: String) = JSONObject("""{"firmwareFence":$body}""")

    @Test
    fun `a served fence carries its radius, its text and its centre`() {
        val fence = firmwareFence(
            view("""{"radiusMetres":300.0,"radiusText":"300 m","centre":{"latitude":41.7,"longitude":44.8}}"""),
        )

        assertNotNull(fence)
        assertEquals(300.0, fence!!.radiusMetres, 0.001)
        assertEquals("300 m", fence.radiusText)
        assertEquals(41.7, fence.centre!!.latitude, 0.001)
    }

    @Test
    fun `no fence at all is not a fence of no size`() {
        assertNull(firmwareFence(JSONObject("""{}""")))
        assertNull(firmwareFence(null))
    }

    @Test
    fun `a radius of zero is absent parameters, not a circle round home`() {
        assertNull(firmwareFence(view("""{"radiusMetres":0.0,"radiusText":"0 m"}""")))
    }

    @Test
    fun `a fence with no home to sit on still reports its radius`() {
        val fence = firmwareFence(view("""{"radiusMetres":300.0,"radiusText":"300 m","centre":null}"""))

        assertNotNull(fence)
        assertNull(fence!!.centre)
        assertEquals("300 m", fence.radiusText)
    }
}
