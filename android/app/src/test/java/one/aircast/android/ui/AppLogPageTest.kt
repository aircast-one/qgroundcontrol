package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class AppLogPageTest {
    @Test
    fun `search text is encoded so commas stay inside one argument`() {
        assertEquals(
            "view.appLog(1,hub,a%2C+b,1,7)",
            appLogPath(AppLogFilter(levelIndex = 2, category = "hub", text = "a, b", regex = true), 7),
        )
        assertEquals("view.appLog(0,,,0,)", appLogPath(AppLogFilter(levelIndex = 0), null))
    }

    @Test
    fun `held entries evicted from the core are dropped before new ones are appended`() {
        val read = appLogRead(
            JSONObject(
                """{"class":"AppLog","levels":[],"categories":[],"first":5,"entries":[""" +
                    """{"sequence":6,"level":2,"message":"W lost","category":"hub","timestamp":"03:20:11.000","source":""}]}""",
            ),
        )!!
        val held = listOf(4L, 5L).map { AppLogEntry(it, 1, "I", "hub", "", "") }
        assertEquals(listOf(5L, 6L), mergedEntries(held, read).map { it.sequence })
    }
}
