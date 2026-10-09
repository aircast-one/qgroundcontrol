package one.aircast.android.ui

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ItemEditorTest {
    @Test
    fun `structure scan note comes from the core and blank means none like StructureScanEditor`() {
        assertEquals("The polygon outlines the structure's surface, not the flight path.", gridNote(JSONObject("""{"gridNote":"The polygon outlines the structure's surface, not the flight path."}""")))
        assertEquals(null, gridNote(JSONObject("""{"gridNote":null}""")))
        assertEquals(null, gridNote(null))
    }

    @Test
    fun `vehicle position mode shows the vehicle's coordinate and frame altitude like EditPositionDialog`() {
        val shown = vehiclePositionOf(JSONObject("""{"valid":true,"latitude":47.3977419,"longitude":8.5455938}"""), JSONObject("""{"kind":"fact","value":12.3,"valueString":"12.3","units":"m"}"""))
        assertEquals(VehiclePosition("47.3977419", "8.5455938", "12.3 m"), shown)
        assertEquals("", vehiclePositionOf(JSONObject("""{"valid":true,"latitude":1.0,"longitude":2.0}"""), null)?.altitude)
        assertEquals(null, vehiclePositionOf(JSONObject("""{"valid":false}"""), null))
        assertEquals(listOf("Alt (Rel)", "Alt (AMSL)", "Alt (AGL)", "Alt (AGL)", null), listOf(1, 2, 3, 4, 0).map(::vehicleAltitudeLabel))
    }

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
    fun `landing altitudes carry the pattern's altitude frame like AltitudeFactTextField`() {
        val fields = itemFields(JSONObject("""{"fields":[
            {"name":"finalApproachAltitude","label":"Altitude","control":"number","units":"m","path":"plan.missionController.visualItems.4.finalApproachAltitude"},
            {"name":"landingAltitude","label":"Altitude","control":"number","units":"m","path":"plan.missionController.visualItems.4.landingAltitude"},
            {"name":"loiterRadius","label":"Radius","control":"number","units":"m","path":"plan.missionController.visualItems.4.loiterRadius"}]}"""))
        assertEquals(listOf("m Rel", "m Rel", "m"), withLandingFrameUnits(fields, true).map { it.units })
        assertEquals(listOf("m AMSL", "m AMSL", "m"), withLandingFrameUnits(fields, false).map { it.units })
        assertEquals(listOf("m", "m", "m"), withLandingFrameUnits(fields, null).map { it.units })
    }

    @Test
    fun `a section header sits above the first row of each landing editor section`() {
        val view = JSONObject("""{"fields":[
            {"path":"a","section":"Final approach"},{"path":"b","section":"Final approach"},
            {"path":"c","section":"Landing point"},{"path":"d","section":"Landing point"},{"path":"e"}]}""")
        assertEquals(mapOf("a" to "Final approach", "c" to "Landing point"), sectionStarts(view))
        assertTrue(sectionStarts(JSONObject("""{"fields":[{"path":"a"}]}""")).isEmpty())
    }

    @Test
    fun `distance and glide slope rows carry the radio button that picks between them`() {
        val view = JSONObject("""{"fields":[
            {"path":"i.landingDistance","choice":{"path":"i.valueSetIsDistance","value":true,"selected":false}},
            {"path":"i.glideSlope","choice":{"path":"i.valueSetIsDistance","value":false,"selected":true}},{"path":"i.landingHeading"}]}""")
        assertEquals(
            mapOf("i.landingDistance" to RadioChoice("i.valueSetIsDistance", true, false), "i.glideSlope" to RadioChoice("i.valueSetIsDistance", false, true)),
            radioChoices(view),
        )
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

    @Test
    fun `a landing pattern's notes come from the core in order`() {
        val view = org.json.JSONObject("""{"landing":true,"landingNotes":["* Actual flight path will vary.","* Avoid tailwind on approach to land."]}""")
        org.junit.Assert.assertEquals(listOf("* Actual flight path will vary.", "* Avoid tailwind on approach to land."), landingNotes(view))
        org.junit.Assert.assertEquals(emptyList<String>(), landingNotes(org.json.JSONObject("{}")))
    }
}
