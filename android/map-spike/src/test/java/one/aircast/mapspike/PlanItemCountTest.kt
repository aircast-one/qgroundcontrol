package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class PlanItemCountTest {
    private fun model(count: Int) =
        JSONObject("""{"kind":"object","items":[${List(count) { "{}" }.joinToString(",")}]}""")

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

    @Test
    fun `a takeoff and a return to launch are named by flag and command`() {
        val plan = JSONObject(
            """{"kind":"object","items":[{},""" +
                """{"kind":"takeoff","name":"Start"},""" +
                """{"kind":"command","command":20,"endsRoute":true,"name":"irrelevant"},""" +
                """{"kind":"waypoint","flownLeg":true}]}""",
        )

        assertEquals(listOf("takeoff", "RTL", "1 after the landing"), planShape(plan))
    }

    @Test
    fun `a plan with neither names nothing`() {
        assertEquals(emptyList<String>(), planShape(model(3)))
        assertEquals(emptyList<String>(), planShape(null))
    }

    @Test
    fun `an item added after the landing is named as such`() {
        val plan = JSONObject(
            """{"kind":"object","items":[{},{"kind":"waypoint","flownLeg":true},""" +
                """{"kind":"command","command":20,"endsRoute":true},{"kind":"waypoint","flownLeg":true},{"kind":"waypoint","flownLeg":true}]}""",
        )

        assertEquals(listOf("RTL", "2 after the landing"), planShape(plan))
    }

    @Test
    fun `a plan that ends at its landing strands nothing`() {
        val plan = JSONObject(
            """{"kind":"object","items":[{},{"kind":"waypoint","flownLeg":true},{"kind":"command","command":20,"endsRoute":true}]}""",
        )

        assertEquals(listOf("RTL"), planShape(plan))
    }
}
