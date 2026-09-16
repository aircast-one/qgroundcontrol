package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.assertNull
import org.junit.Test

class LandingCutTest {
    private fun plan(vararg items: String) =
        JSONObject("""{"kind":"object","items":[${items.joinToString(",")}]}""")

    private fun placed(longitude: Double) =
        """{"kind":"waypoint","flownLeg":true,""" +
            """"coordinate":{"latitude":41.0,"longitude":$longitude}}"""

    private val rtl = """{"kind":"command","command":20,"endsRoute":true}"""
    private val landed = """{"kind":"land","endsRoute":true}"""
    private val settings = """{"kind":"settings","flownLeg":true,""" +
        """"coordinate":{"latitude":41.0,"longitude":44.0}}"""

    @Test
    fun `an undrawable landing still cuts the route`() {
        val items = missionItems(plan(settings, placed(44.1), rtl, placed(44.2)))

        assertEquals(3, items.size)
        assertTrue(items[1].routed)
        assertFalse(items[2].routed)
    }

    @Test
    fun `nothing is after the landing when there is no landing`() {
        assertTrue(missionItems(plan(settings, placed(44.1), placed(44.2))).all { it.routed })
    }

    @Test
    fun `the route is cut at the item the core marked as ending it`() {
        val items = plan(settings, placed(44.1), rtl, placed(44.2)).optJSONArray("items")

        assertEquals(2, routeEndsAfter(items))
        assertEquals(Int.MAX_VALUE, routeEndsAfter(null))
    }

    @Test
    fun `a landing and a return both end the route, and the head does not tell them apart`() {
        val byCommand = plan(settings, placed(44.1), rtl, placed(44.2)).optJSONArray("items")
        val byLanding = plan(settings, placed(44.1), landed, placed(44.2)).optJSONArray("items")

        assertEquals(2, routeEndsAfter(byCommand))
        assertEquals(2, routeEndsAfter(byLanding))
    }

    @Test
    fun `an item the core says is not a flown leg is never routed`() {
        val roi = """{"kind":"roi","flownLeg":false,""" +
            """"coordinate":{"latitude":41.0,"longitude":44.9}}"""
        val items = missionItems(plan(settings, placed(44.1), roi, placed(44.2)))

        assertFalse(items.single { it.command == "" && it.longitude == 44.9 }.routed)
        assertTrue(items.first { it.longitude == 44.1 }.routed)
    }

    @Test
    fun `the stray items after a landing are counted in the plan shape`() {
        assertEquals(
            listOf("RTL", "1 after the landing"),
            planShape(plan(settings, rtl, placed(44.2))),
        )
    }

    @Test
    fun `an item with no usable coordinate is not plotted`() {
        assertNull("Mission Start carries no coordinate at all", placed(JSONObject("""{"kind":"x"}"""), "coordinate"))
        assertNull(
            "a coordinate the core could not resolve arrives as nulls, and NaN is not plottable",
            placed(JSONObject("""{"coordinate":{"latitude":null,"longitude":null}}"""), "coordinate"),
        )
    }

}
