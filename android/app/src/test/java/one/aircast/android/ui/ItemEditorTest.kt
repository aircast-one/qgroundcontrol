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

    @Test
    fun `the speed section is offered only where the item has one`() {
        val speed = speedSection(JSONObject("""{"speedSection":{"available":true,"specified":true,"value":8.5,"units":"m/s","path":"p","specifyPath":"s"}}"""))!!
        assertTrue(speed.specified)
        assertEquals(8.5, speed.value!!, 0.0)
        assertEquals(null, speedSection(JSONObject("""{"speedSection":{"available":false}}""")))
        assertEquals(null, speedSection(JSONObject("""{"speedSection":null}""")))
    }

    @Test
    fun `position forms seed the UTM and MGRS fields`() {
        val forms = positionForms(JSONObject("""{"utm":{"zone":32,"southern":false,"easting":464617.5,"northing":5247152.25},"mgrs":"32T MN 64617 47152"}"""))!!
        assertEquals("32", forms.zone)
        assertEquals("464617.50", forms.easting)
        assertEquals("32T MN 64617 47152", forms.mgrs)
        assertEquals("view.utmToGeo(1,2,32,true)", utmToGeoPath("1", "2", "32", true))
        assertEquals(47.0 to 8.0, geoOf(JSONObject("""{"valid":true,"latitude":47.0,"longitude":8.0}""")))
        assertEquals(null, geoOf(JSONObject("""{"valid":false}""")))
    }
}
