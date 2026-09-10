package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.assertNull
import org.junit.Test

class ParameterFormTest {
    @Test
    fun `a fact response becomes a fact addressed by its parameter path`() {
        val json = JSONObject(
            """
            {"kind":"fact","name":"RTL_ALT","shortDescription":"Return altitude",
             "units":"cm","valueString":"1500","value":1500,"typeIsBool":false,"readOnly":false}
            """.trimIndent(),
        )
        val fact = factFromParameter("RTL_ALT", json)!!
        assertEquals(parameterPath("RTL_ALT"), fact.path)
        assertEquals("RTL_ALT", fact.name)
        assertEquals("Return altitude", fact.description)
        assertEquals("cm", fact.units)
    }

    @Test
    fun `a parameter with no description is labelled by its name`() {
        val json = JSONObject("""{"kind":"fact","name":"","shortDescription":"","valueString":"0"}""")
        val fact = factFromParameter("MNT_ANGMIN_PAN", json)!!
        assertEquals("MNT_ANGMIN_PAN", fact.name)
        assertEquals("MNT_ANGMIN_PAN", fact.title)
    }

    @Test
    fun `a description still wins over the name`() {
        val json = JSONObject("""{"kind":"fact","name":"RTL_ALT","shortDescription":"Return altitude"}""")
        assertEquals("Return altitude", factFromParameter("RTL_ALT", json)!!.title)
    }

    @Test
    fun `a parameter the vehicle does not have is skipped`() {
        assertNull(factFromParameter("NOPE", JSONObject("""{"kind":"value","value":null}""")))
        assertNull(factFromParameter("NOPE", JSONObject("""{"kind":"null"}""")))
        assertNull(factFromParameter("NOPE", JSONObject("{}")))
    }









    @Test
    fun `an empty error from QGC means the value is acceptable`() {
        assertNull(validationMessage(""))
        assertNull(validationMessage("   "))
    }

    @Test
    fun `QGC's own wording is passed through unchanged`() {
        assertEquals(
            "Value must be within 0 and 100",
            validationMessage("Value must be within 0 and 100"),
        )
    }

    @Test
    fun `a bridge call that returned nothing does not block the write`() {
        assertNull(validationMessage(null))
    }

    @Test
    fun `a non-string result is not treated as an error`() {
        assertNull(validationMessage(false))
        assertNull(validationMessage(0))
    }

    private fun ranged(
        min: String = "", max: String = "",
        minDefault: Boolean = true, maxDefault: Boolean = true,
        default: String = "",
        vehicleReboot: Boolean = false, qgcReboot: Boolean = false,
    ) = Fact(
        path = "p", name = "p", description = "", units = "", valueString = "1",
        value = 1, enumStrings = emptyList(), enumIndex = 0,
        isBool = false, isString = false, readOnly = false,
        minString = min, maxString = max,
        minIsDefaultForType = minDefault, maxIsDefaultForType = maxDefault,
        defaultValueString = default,
        vehicleRebootRequired = vehicleReboot, qgcRebootRequired = qgcReboot,
    )

    @Test
    fun `a limit that is only the type's own limit is not shown`() {
        assertNull(factConstraintNote(ranged(min = "0", max = "4294967295")))
    }

    @Test
    fun `a real constraint is shown`() {
        assertEquals("Max 100", factConstraintNote(ranged(max = "100", maxDefault = false)))
    }

    @Test
    fun `min max and default read together`() {
        assertEquals(
            "Min 6 · Max 48 · Default 14",
            factConstraintNote(
                ranged(min = "6", max = "48", minDefault = false, maxDefault = false, default = "14"),
            ),
        )
    }

    @Test
    fun `a vehicle reboot is named ahead of an app restart`() {
        assertEquals(
            "Reboot the vehicle for this to take effect.",
            factRebootNote(ranged(vehicleReboot = true, qgcReboot = true)),
        )
    }

    @Test
    fun `an app restart is named when only that is required`() {
        assertEquals(
            "Restart Aircast for this to take effect.",
            factRebootNote(ranged(qgcReboot = true)),
        )
    }

    @Test
    fun `a parameter needing no restart says nothing`() {
        assertNull(factRebootNote(ranged()))
    }

    @Test
    fun `a bitmask on a setup page names its bits instead of showing a number`() {
        val control = JSONObject(
            """{"control":"bitmask","name":"SIMPLE","label":"Simple mode bitmask","path":"p",
                "value":5,"valueString":"5","display":"5",
                "bits":[{"label":"SwitchPos1","raw":"1","set":true},
                        {"label":"SwitchPos2","raw":"2","set":false},
                        {"label":"SwitchPos3","raw":"4","set":true}]}"""
        )

        val fact = factFromControl(control)!!
        assertTrue(fact.isBitmask)
        assertEquals(listOf(1L, 2L, 4L), fact.bitmaskValues)
        assertEquals("SwitchPos1, SwitchPos3", bitmaskSummary(fact))
    }

    @Test
    fun `a fact the core called a choice stays a choice even when it carries bits`() {
        val control = JSONObject(
            """{"control":"choice","name":"FS_OPTIONS","label":"Failsafe options","path":"p",
                "value":1,"valueString":"1","display":"Continue",
                "options":[{"label":"None","raw":"0"},{"label":"Continue","raw":"1"}],
                "bits":[{"label":"RC","raw":"1","set":true}]}"""
        )

        val fact = factFromControl(control)!!
        assertFalse("the core decides the control kind, the head does not re-derive it", fact.isBitmask)
        assertTrue(fact.isEnum)
    }
}

class ReadOnlyNoteTest {
    private fun fact(name: String, readOnly: Boolean) = one.aircast.android.bridge.Fact(
        path = "p/$name", name = name, description = "", units = "",
        valueString = "0", value = 0, enumStrings = emptyList(), enumIndex = -1,
        isBool = false, isString = false, readOnly = readOnly,
    )

    @Test
    fun `nothing read-only says nothing`() {
        assertNull(readOnlyNote(listOf(fact("FLTMODE1", false), fact("FLTMODE2", false))))
    }

    @Test
    fun `all read-only says so without naming them`() {
        assertEquals(
            "This firmware reports all of these as read-only, so they are shown for reference.",
            readOnlyNote(listOf(fact("A", true), fact("B", true))),
        )
    }

    @Test
    fun `some read-only names which ones`() {
        assertEquals(
            "This firmware reports FLTMODE2, FLTMODE3 as read-only, " +
                "so they are shown but cannot be changed here.",
            readOnlyNote(
                listOf(fact("FLTMODE1", false), fact("FLTMODE2", true), fact("FLTMODE3", true)),
            ),
        )
    }

    @Test
    fun `the setup pages come from the core, grouped and flagged`() {
        val view = JSONObject("""{"groups":[
            {"title":"Vehicle","pages":[
              {"name":"Frame","native":true,"parameterSections":true},
              {"name":"Motors","native":false,"parameterSections":false}]},
            {"title":"Support","pages":[
              {"name":"Remote Support","native":true,"parameterSections":false}]}]}""")

        assertEquals(listOf("Vehicle", "Support"), setupGroups(view).map { it.title })
        assertEquals(true, setupPage(view, "Frame")?.native)
        assertEquals(true, setupPage(view, "Frame")?.parameterSections)
        assertEquals(false, setupPage(view, "Motors")?.native)
        assertEquals(false, setupPage(view, "Remote Support")?.parameterSections)
        assertNull(setupPage(view, "A page this firmware does not have"))
    }

    @Test
    fun `a control becomes a row with the core's label and options`() {
        val fact = factFromControl(
            JSONObject("""{"class":"Control","control":"choice","label":"Frame Class",
                "name":"FRAME_CLASS","display":"Quad","value":1,"valueString":"1","units":"",
                "path":"vehicle.parameterManager.getParameter(-1,FRAME_CLASS)","readOnly":false,
                "rebootRequired":true,
                "options":[{"label":"Undefined","raw":"0"},{"label":"Quad","raw":"1"}]}"""),
        )!!

        assertEquals("Frame Class", fact.title)
        assertEquals(listOf("Undefined", "Quad"), fact.enumStrings)
        assertEquals(1, fact.enumIndex)
        assertEquals(true, fact.vehicleRebootRequired)
    }

    @Test
    fun `a control the vehicle does not have is dropped rather than shown blank`() {
        assertNull(
            factFromControl(
                JSONObject("""{"class":"Control","control":"number","label":"","name":"",
                    "display":"0","value":0,"valueString":"0","units":"",
                    "path":"vehicle.parameterManager.getParameter(-1,BATT_MONITOR)"}"""),
            ),
        )
    }

    @Test
    fun `the page path carries the name and no separators the watch would split`() {
        assertEquals("view.setup(Flight Modes)", setupPagePath("Flight Modes"))
        assertEquals(false, setupPagePath("Flight Modes").contains(","))
    }
}
