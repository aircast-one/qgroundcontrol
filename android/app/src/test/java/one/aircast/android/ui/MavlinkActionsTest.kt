package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MavlinkActionsTest {
    @Test
    fun the_files_and_fly_view_actions_read_from_the_core() {
        assertNull(mavlinkActions(JSONObject("{}")))
        val read = mavlinkActions(JSONObject("""{"files":["fly.json"],"flyViewFile":"fly.json","joystickFile":"","flyViewPath":"a","joystickPath":"b","actions":[{"label":"Lights","description":"toggle"}]}"""))!!
        assertEquals(listOf(MavlinkActionEntry("Lights", "toggle")), read.actions)
        assertEquals("", chosenFile(NO_ACTIONS_FILE))
        assertEquals("fly.json", chosenFile("fly.json"))
    }
}
