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
                    """"names":["Survey","Corridor Scan","Structure Scan","No Template"]}}""",
            ),
        )
        assertEquals(PlanTemplatesState(true, false, "Click in map to set position", listOf("Survey", "Corridor Scan", "Structure Scan", "No Template")), read)
        assertNull(planTemplates(JSONObject("{}")))
    }
}
