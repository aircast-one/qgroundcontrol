package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class ItemEditorTitleTest {
    @Test
    fun `the editor heads with the sequence and command name like MissionItemEditor`() {
        assertEquals("#3 Waypoint", itemEditorTitle(JSONObject("""{"commandName":"Waypoint","sequenceNumber":3}"""), 2))
        assertEquals("Item 2", itemEditorTitle(null, 2))
    }
}
