package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class PlanItemCountTest {
    private fun model(count: Int) =
        JSONObject("""{"kind":"object","elements":[${List(count) { "{}" }.joinToString(",")}]}""")

    @Test
    fun `the settings item is not something the pilot added`() {
        assertEquals(0, planItemCount(model(1)))
        assertEquals(3, planItemCount(model(4)))
    }

    @Test
    fun `a plan that could not be read counts nothing rather than going negative`() {
        assertEquals(0, planItemCount(null))
        assertEquals(0, planItemCount(model(0)))
    }
}
