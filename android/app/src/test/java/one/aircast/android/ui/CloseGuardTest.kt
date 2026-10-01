package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class CloseGuardTest {
    @Test
    fun `close prompts are read in the order the core asks them`() {
        val read = closePrompts(JSONObject("""{"prompts":[{"id":"unsavedMission","message":"A"},{"id":"activeConnections","message":"B"}]}"""))
        assertEquals(listOf("A", "B"), read)
        assertEquals(emptyList<String>(), closePrompts(null))
    }
}
