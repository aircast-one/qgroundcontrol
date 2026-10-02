package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class ItemCommandTest {
    @Test
    fun `the editor note is the command description unless raw edit is on, like SimpleItemEditor`() {
        val view = JSONObject("""{"commandDescription":"Travel to a position in 3D space."}""")
        assertEquals("Travel to a position in 3D space.", itemNote(view, rawOn = false))
        assertEquals(RAW_EDIT_NOTE, itemNote(view, rawOn = true))
        assertEquals(null, itemNote(JSONObject("""{"commandDescription":null}"""), rawOn = false))
    }

    @Test
    fun `a takeoff item offers no command change, like MissionItemEditor`() {
        assertEquals(
            listOf(true, false, false),
            listOf("""{"simple":true,"takeoff":false}""", """{"simple":true,"takeoff":true}""", """{"simple":false}""").map { commandEditable(JSONObject(it)) },
        )
    }

    @Test
    fun `the picker opens on the item's category, like MissionCommandDialog`() {
        assertEquals("Advanced", startCategory(listOf("Basic", "Advanced"), "Advanced"))
        assertEquals("Basic", startCategory(listOf("Basic", "Advanced"), "Gone"))
        assertEquals("Basic", startCategory(listOf("Basic", "Advanced"), null))
    }

    @Test
    fun `a plane's new takeoff shows the climb-out step until Done, like SimpleItemEditor's wizard`() {
        val lines = wizardLines(JSONObject("""{"wizardMode":true,"wizardText":["Move 'T' Takeoff to the climbout location.","Ensure clear of obstacles and into the wind."]}"""))
        assertEquals(listOf("Move 'T' Takeoff to the climbout location.", "Ensure clear of obstacles and into the wind."), lines)
        assertEquals(emptyList<String>(), wizardLines(JSONObject("""{"wizardMode":false,"wizardText":["x"]}""")))
        assertEquals("plan.missionController.visualItems.1.wizardMode", wizardModePath(1))
    }
}
