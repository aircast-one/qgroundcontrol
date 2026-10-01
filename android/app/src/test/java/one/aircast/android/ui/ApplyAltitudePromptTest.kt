package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ApplyAltitudePromptTest {
    @Test
    fun `the prompt shows only while the core asks`() {
        val asked = applyAltitudePrompt(JSONObject("""{"applyAltitudePrompt":{"title":"Apply new altitude","text":"t"}}"""))
        assertEquals(AltitudePrompt("Apply new altitude", "t"), asked)
        assertNull(applyAltitudePrompt(JSONObject("""{"applyAltitudePrompt":null}""")))
    }
}
