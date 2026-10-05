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
        assertEquals(PlanTemplatesState(true, false, "Click in map to set position", listOf("Survey", "Corridor Scan", "Structure Scan", "No Template"), false, "No Template"), read)
        assertNull(planTemplates(JSONObject("{}")))
    }

    @Test
    fun `the prompt speaks of taps on a touch screen`() {
        assertEquals("Tap in map to set position", touchWording("Click in map to set position"))
        assertEquals("Drag to move home position. Tap to set new position.", touchWording("Drag to move home position. Click to set new position."))
    }

    @Test
    fun `the blank mission comes first and the prompt names one first step until home is set`() {
        val state = PlanTemplatesState(true, true, "Click in map to set position", listOf("Survey", "Corridor Scan", "No Template"), false, "No Template")
        assertEquals(listOf("No Template" to "Blank mission", "Survey" to "Survey", "Corridor Scan" to "Corridor scan"), templateChoices(state))
        assertEquals("Tap the map to set home, or start from a template", templatePrompt(state))
        assertEquals(
            "Drag to move home position. Tap to set new position.",
            templatePrompt(state.copy(homeSet = true, prompt = "Drag to move home position. Click to set new position.")),
        )
    }
}
