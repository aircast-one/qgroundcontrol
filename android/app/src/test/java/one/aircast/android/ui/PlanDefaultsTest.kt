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
        assertTrue(planDefaultsNote(emptyList()).contains("does not report"))
    }

    @Test
    fun `with defaults present the note explains what they do`() {
        val facts = planDefaults(plan("""{"altitude":${control("a", 50.0, "m")}}"""))

        assertTrue(planDefaultsNote(facts).contains("New mission items"))
    }
}
