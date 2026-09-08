package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class LandingCutTest {
    private fun plan(vararg elements: String) =
        JSONObject("""{"kind":"object","elements":[${elements.joinToString(",")}]}""")

    private fun placed(longitude: Double) =
        """{"specifiesCoordinate":true,"amslEntryAlt":50.0,""" +
            """"coordinate":{"latitude":41.0,"longitude":$longitude}}"""

    private val rtl = """{"specifiesCoordinate":false,"command":20}"""
    private val landed = """{"specifiesCoordinate":false,"isLandCommand":true}"""
    private val settings = """{"specifiesCoordinate":true,"amslEntryAlt":0.0,""" +
        """"coordinate":{"latitude":41.0,"longitude":44.0}}"""

    @Test
    fun `an undrawable landing still cuts the route`() {
        val items = missionItems(plan(settings, placed(44.1), rtl, placed(44.2)))

        assertEquals(3, items.size)
        assertFalse(items[1].afterLanding)
        assertTrue(items[2].afterLanding)
    }

    @Test
    fun `nothing is after the landing when there is no landing`() {
        val items = missionItems(plan(settings, placed(44.1), placed(44.2)))

        assertTrue(items.none { it.afterLanding })
    }

    @Test
    fun `the distance stops at the landing too`() {
        val toLanding = terrainProfile(plan(settings, placed(44.1), rtl, placed(44.9)))
        val withoutTheStrayItem = terrainProfile(plan(settings, placed(44.1), rtl))

        assertEquals(withoutTheStrayItem.distance, toLanding.distance, 1e-9)
    }

    @Test
    fun `the map and the distance cut at the same element`() {
        val elements = plan(settings, placed(44.1), rtl, placed(44.2)).optJSONArray("elements")

        assertEquals(2, routeEndsAfter(elements))
        assertEquals(Int.MAX_VALUE, routeEndsAfter(null))
    }

    @Test
    fun `an RTL ends the route even though it is not a land command`() {
        val byCommand = plan(settings, placed(44.1), rtl, placed(44.2)).optJSONArray("elements")
        val byFlag = plan(settings, placed(44.1), landed, placed(44.2)).optJSONArray("elements")

        assertEquals(2, routeEndsAfter(byCommand))
        assertEquals(2, routeEndsAfter(byFlag))
    }

    // QGC's source adds "the final segment back to home" when it finds an RTL,
    // and on the handset it does not: a survey plus an RTL reported 5896 m in the
    // panel, the survey's own distance, with no leg back to the launch point.
    // Whatever gates it upstream, the measurement is what this has to match.
    @Test
    fun `an RTL ends the route without adding a leg home`() {
        val homeward = terrainProfile(plan(settings, placed(44.1), rtl))
        val stopping = terrainProfile(plan(settings, placed(44.1), landed))

        assertEquals(stopping.distance, homeward.distance, 1e-9)
    }
}
