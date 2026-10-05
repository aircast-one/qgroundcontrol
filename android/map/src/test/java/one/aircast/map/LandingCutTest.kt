package one.aircast.map

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
    fun `only a return ends the route, a landing just skips the leg after it`() {
        val byCommand = plan(settings, placed(44.1), rtl, placed(44.2)).optJSONArray("items")
        val byLanding = plan(settings, placed(44.1), landed, placed(44.2)).optJSONArray("items")

        assertEquals(2, routeEndsAfter(byCommand))
        assertEquals("MissionController breaks only at an RTL", Int.MAX_VALUE, routeEndsAfter(byLanding))
    }

    @Test
    fun `the leg out of a landing is not drawn, the ones after it are`() {
        val land = """{"kind":"land","endsRoute":true,"command":21,"flownLeg":true,""" +
            """"coordinate":{"latitude":41.0,"longitude":44.15}}"""
        val items = missionItems(plan(settings, placed(44.1), land, placed(44.2), placed(44.3)))
        assertEquals(setOf(3), legsAfterLanding(plan(settings, placed(44.1), land, placed(44.2), placed(44.3)).optJSONArray("items")))
        val path = missionPath(items, false)?.geometry() as org.maplibre.geojson.MultiLineString
        assertEquals("Don't draw segments immediately after a landing item", listOf(listOf(44.1, 44.15), listOf(44.2, 44.3)), path.coordinates().map { line -> line.map { it.longitude() } })
        assertTrue(legArrows(items, false).none { it.at.longitude in 44.15..44.2 })
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
            listOf("RTL", "1 after RTL"),
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
