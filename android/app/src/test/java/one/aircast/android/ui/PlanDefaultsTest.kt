package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PlanDefaultsTest {

    private fun control(name: String, value: Double, units: String) =
        """{"kind":"object","class":"Control","control":"number","name":"$name","label":"$name",
            "path":"settings.appSettings.$name","value":$value,"valueString":"$value",
            "units":"$units","enabled":true,"readOnly":false,"options":[],"bits":[]}"""

    private fun plan(defaults: String) = JSONObject(
        """{"kind":"object","class":"Plan","defaults":$defaults}""",
    )

    @Test
    fun `the three the core serves are the three offered`() {
        val facts = planDefaults(
            plan(
                """{"altitude":${control("defaultMissionItemAltitude", 50.0, "m")},
                    "cruise":${control("offlineEditingCruiseSpeed", 15.0, "m/s")},
                    "hover":${control("offlineEditingHoverSpeed", 5.0, "m/s")},
                    "speedUnits":"m/s"}""",
            ),
        )

        assertEquals(
            "the altitude every new mission item starts at, and the speeds a plan edited with no " +
                "vehicle is flown at - none of which was on any screen, so an operator adding a " +
                "waypoint got a height they could not see or change from the Plan tab",
            listOf("defaultMissionItemAltitude", "offlineEditingCruiseSpeed", "offlineEditingHoverSpeed"),
            facts.map { it.name },
        )
    }

    @Test
    fun `the order is the list's, not whatever order the keys arrive in`() {
        val facts = planDefaults(
            plan(
                """{"hover":${control("hover", 5.0, "m/s")},
                    "cruise":${control("cruise", 15.0, "m/s")},
                    "altitude":${control("altitude", 50.0, "m")}}""",
            ),
        )

        assertEquals(
            "JSONObject.keys() has no defined order on Android, so height before speeds is what " +
                "PLAN_DEFAULT_KEYS is for - the filtering is structural and would work without it",
            listOf("altitude", "cruise", "hover"),
            facts.map { it.name },
        )
    }

    @Test
    fun `a value that is not an object cannot become a control`() {
        val facts = planDefaults(
            plan("""{"altitude":${control("a", 50.0, "m")},"speedUnits":"ft/s"}"""),
        )

        assertEquals(1, facts.size)
        assertEquals("a", facts.single().name)
    }

    @Test
    fun `a core too old to serve defaults says so rather than showing an empty dialog`() {
        assertEquals(emptyList<Any>(), planDefaults(JSONObject("""{"kind":"object"}""")))
        assertEquals(emptyList<Any>(), planDefaults(null))
        assertTrue(planDefaultsNote(null, emptyList()).contains("does not report"))
    }

    @Test
    fun `with defaults present the note is the core's speed note, like MissionSettingsEditor`() {
        val view = plan("""{"altitude":${control("a", 50.0, "m")},"speedNote":"Speeds are used to estimate mission time only. They do not change the flight speed."}""")

        assertEquals("Speeds are used to estimate mission time only. They do not change the flight speed.", planDefaultsNote(view, planDefaults(view)))
        val noSpeeds = plan("""{"altitude":${control("a", 50.0, "m")},"speedNote":null}""")
        assertEquals("", planDefaultsNote(noSpeeds, planDefaults(noSpeeds)))
    }

    @Test
    fun `the mission flight speed is the settings item's speed section with SpeedSection's user range`() {
        val view = plan("""{"flightSpeed":{"available":true,"specified":true,"value":7.5,"units":"m/s","slider":{"from":0.0,"to":30.0,"decimals":1},
            "path":"plan.missionController.visualItems.0.speedSection.flightSpeed","specifyPath":"plan.missionController.visualItems.0.speedSection.specifyFlightSpeed"}}""")
        val speed = speedSectionOf(view.optJSONObject("defaults")?.optJSONObject("flightSpeed"))!!
        assertEquals(one.aircast.android.bridge.FactSlider(0f, 30f, 1, ""), speed.slider)
        assertEquals(7.5, speed.value!!, 0.0)
        assertEquals(null, speedSectionOf(JSONObject("""{"available":false}""")))
    }

    @Test
    fun `a default carrying the user range gets the slider MissionDefaultsEditor draws under its field`() {
        val ranged = control("offlineEditingCruiseSpeed", 15.0, "m/s").replace("\"bits\":[]", "\"bits\":[],\"slider\":{\"from\":1.0,\"to\":30.0,\"decimals\":1}")
        val facts = planDefaults(plan("""{"cruise":$ranged,"altitude":${control("defaultMissionItemAltitude", 50.0, "m")}}"""))

        assertEquals(one.aircast.android.bridge.FactSlider(1f, 30f, 1, ""), facts.last().slider)
        assertEquals(null, facts.first().slider)
    }
}
