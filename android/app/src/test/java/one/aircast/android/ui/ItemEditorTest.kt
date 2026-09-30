package one.aircast.android.ui

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ItemEditorTest {
    @Test
    fun `commands and categories read from the command tree`() {
        val commands = commandChoices(JSONArray("""[{"command":16,"friendlyName":"Waypoint","description":"Travel to a position in 3D space."},{"command":19,"friendlyName":"Loiter (time)","description":"d"}]"""))
        assertEquals(listOf(16, 19), commands.map { it.id })
        assertEquals("Waypoint", commands[0].name)
        assertEquals(listOf("Basic", "Advanced"), categoryNames(JSONArray("""["Basic","Advanced"]""")))
        assertTrue(commandChoices(null).isEmpty())
    }

    @Test
    fun `an item's command is written on its own path`() {
        assertEquals("plan.missionController.visualItems.3.command", itemCommandPath(3))
        assertEquals("view.itemFacts(3)", itemFactsPath(3))
        assertEquals(1, itemFields(JSONObject("""{"fields":[{"name":"Hold","label":"Hold","control":"number","path":"p"}]}""")).size)
    }
}
