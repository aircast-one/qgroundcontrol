package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PlanTemplatesTest {
    @Test
    fun `templates read from the plan view in the order QGC offers them`() {
        val read = planTemplates(
            JSONObject(
                """{"templates":{"show":true,"enabled":false,"prompt":"Click in map to set position",""" +
                    """"names":["Survey","Corridor Scan","Structure Scan","No Template"],"homeSet":false,"blank":"No Template"}}""",
            ),
        )
        assertEquals(PlanTemplatesState(false, listOf("Survey", "Corridor Scan", "Structure Scan", "No Template"), "No Template"), read)
        assertNull(planTemplates(JSONObject("{}")))
    }

    @Test
    fun `the blank mission comes first and stands for a new empty plan`() {
        val state = PlanTemplatesState(true, listOf("Survey", "Corridor Scan", "No Template"), "No Template")
        assertEquals(listOf(null to "Blank mission", "Survey" to "Survey", "Corridor Scan" to "Corridor scan"), templateChoices(state))
    }

    @Test
    fun `with no answer from the core a blank plan is still offered`() {
        assertEquals(listOf(null to "Blank mission"), templateChoices(null))
    }
}
