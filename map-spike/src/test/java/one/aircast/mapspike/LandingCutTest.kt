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
        assertTrue(items[1].routed)
        assertFalse(items[2].routed)
    }

    @Test
    fun `nothing is after the landing when there is no landing`() {
        val items = missionItems(plan(settings, placed(44.1), placed(44.2)))

        assertTrue(items.all { it.routed })
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

    @Test
    fun `an RTL ends the route without adding a leg home`() {
        val homeward = terrainProfile(plan(settings, placed(44.1), rtl))
        val stopping = terrainProfile(plan(settings, placed(44.1), landed))

        assertEquals(stopping.distance, homeward.distance, 1e-9)
    }

    @Test
    fun `a camera target is not a leg in the distance either`() {
        val roi = """{"specifiesCoordinate":true,"isStandaloneCoordinate":true,""" +
            """"amslEntryAlt":50.0,"coordinate":{"latitude":41.0,"longitude":44.9}}"""
        val withRoi = terrainProfile(plan(settings, placed(44.1), roi, placed(44.2)))
        val without = terrainProfile(plan(settings, placed(44.1), placed(44.2)))

        assertEquals(without.distance, withRoi.distance, 1e-9)
    }

    @Test
    fun `an item still being set up is not a leg either`() {
        val halfMade = """{"specifiesCoordinate":true,"isIncomplete":true,""" +
            """"amslEntryAlt":50.0,"coordinate":{"latitude":41.0,"longitude":44.9}}"""
        val withIt = terrainProfile(plan(settings, placed(44.1), halfMade, placed(44.2)))
        val without = terrainProfile(plan(settings, placed(44.1), placed(44.2)))

        assertEquals(without.distance, withIt.distance, 1e-9)
    }

    @Test
    fun `the map and the profile ask the same question about a leg`() {
        val roi = JSONObject("""{"specifiesCoordinate":true,"isStandaloneCoordinate":true}""")
        val halfMade = JSONObject("""{"specifiesCoordinate":true,"isIncomplete":true}""")
        val placeless = JSONObject("""{"specifiesCoordinate":false}""")
        val ordinary = JSONObject("""{"specifiesCoordinate":true}""")

        assertFalse(isFlownLeg(roi))
        assertFalse(isFlownLeg(halfMade))
        assertFalse(isFlownLeg(placeless))
        assertFalse(isFlownLeg(null))
        assertTrue(isFlownLeg(ordinary))
    }
}
