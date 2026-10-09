package one.aircast.android.ui

import one.aircast.map.PlanStat
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RouteSettingsTest {
    private val stats = listOf(PlanStat("Items", "3"), PlanStat("Distance", "93 m"), PlanStat("Time", "00:35"), PlanStat("Max alt", "50.0 m"))

    @Test
    fun `the title pill reads the route at a glance`() {
        assertEquals("3 items · 93 m · 00:35 · max 50.0 m", planStatsLine(stats))
        assertEquals("1 item", planStatsLine(listOf(PlanStat("Items", "1"))))
        assertEquals("", planStatsLine(listOf(PlanStat("Items", "0"))))
    }

    @Test
    fun `a transfer in flight takes the line, and an empty plan falls back to the core's status`() {
        assertEquals("Uploading", headerLine("Uploading", syncing = true, stats))
        assertEquals("Empty plan", headerLine("Empty plan", syncing = false, listOf(PlanStat("Items", "0"), PlanStat("Distance", "0 m"), PlanStat("Time", "00:00"))))
    }

    @Test
    fun `the route altitude is the default every new waypoint takes`() {
        val plan = JSONObject("""{"defaults":{"altitude":{"label":"Default altitude","path":"settings.appSettings.defaultMissionItemAltitude","value":60.0,"units":"m"}}}""")
        assertEquals(RouteAltitude(60.0, "m", "settings.appSettings.defaultMissionItemAltitude"), routeAltitude(plan))
        assertNull(routeAltitude(JSONObject("{}")))
    }

    @Test
    fun `the hold has its own row, so the editor below does not repeat it`() {
        val hold = fact("Hold")
        val accept = fact("Acceptance radius")
        assertEquals(listOf(accept), withoutHeroFields(listOf(hold, accept), JSONObject("""{"hold":{"value":0,"units":"s","path":"p"}}""")))
        assertEquals(listOf(hold, accept), withoutHeroFields(listOf(hold, accept), JSONObject("""{"hold":null}""")))
    }

    private fun fact(label: String) = factFromControl(JSONObject().put("label", label).put("path", label))!!
}
