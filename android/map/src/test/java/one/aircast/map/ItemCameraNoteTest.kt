package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ItemCameraNoteTest {
    @Test
    fun `the mission start note is shown when the core serves it, like MissionSettingsEditor`() {
        val note = "Camera commands above take effect immediately at mission start."
        assertEquals(note, itemCameraNote(JSONObject("""{"available":true,"note":"$note"}""")))
        assertNull(itemCameraNote(JSONObject("""{"available":true,"note":null}""")))
        assertNull(itemCameraNote(JSONObject("""{"available":false,"note":"$note"}""")))
    }
}
