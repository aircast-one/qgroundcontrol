package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PlanFilesViewTest {
    @Test
    fun `patterns are offered by the canonical name the insert takes`() {
        val view = JSONObject(
            """{"patterns":[{"name":"Survey","title":"Survey"},{"name":"Corridor Scan","title":"Korridor-Scan"},{"name":"","title":"x"}]}""",
        )
        assertEquals(listOf("Survey", "Corridor Scan"), patternNames(view))
        assertEquals(emptyList<String>(), patternNames(null))
    }

    @Test
    fun `the plan file is the whole path, and none is null`() {
        assertEquals("/plans/ridge.plan", currentPlanPath(JSONObject("""{"filePath":"/plans/ridge.plan"}""")))
        assertNull(currentPlanPath(JSONObject("""{"filePath":null}""")))
        assertNull(currentPlanPath(null))
    }
}
