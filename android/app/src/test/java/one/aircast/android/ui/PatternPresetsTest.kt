package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PatternPresetsTest {
    @Test
    fun `presets show for surveys and corridors only`() {
        assertEquals("survey", presetKind(JSONObject("""{"presetKind":"survey"}""")))
        assertNull(presetKind(JSONObject("""{"presetKind":null}""")))
        assertNull(presetKind(JSONObject("""{}""")))
        assertEquals(listOf("Mapping 80m", "Inspection"), presetNames(JSONObject("""{"names":["Mapping 80m","Inspection"]}""")))
    }
}
