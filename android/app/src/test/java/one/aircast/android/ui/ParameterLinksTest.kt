package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ParameterLinksTest {
    @Test
    fun `a param link names the parameter and other links are left to the browser`() {
        assertEquals("COM_ARM_WO_GPS", paramLinkName("param://COM_ARM_WO_GPS"))
        assertNull(paramLinkName("param://"))
        assertNull(paramLinkName("https://docs.px4.io"))
    }

    @Test
    fun `a read-only parameter says so until force edit is on`() {
        assertNull(forceEditNote(readOnly = false, forced = false))
        assertEquals(READ_ONLY_NOTE, forceEditNote(readOnly = true, forced = false))
        assertEquals(FORCE_EDIT_NOTE, forceEditNote(readOnly = true, forced = true))
    }

    @Test
    fun `reset offers the default only when the metadata has one`() {
        assertEquals(20, parameterDefault(org.json.JSONObject("""{"defaultValueAvailable":true,"defaultValue":20}""")))
        assertNull(parameterDefault(org.json.JSONObject("""{"defaultValueAvailable":false,"defaultValue":20}""")))
        assertNull(parameterDefault(org.json.JSONObject("""{"defaultValueAvailable":true,"defaultValue":null}""")))
    }

    @Test
    fun `manual entry drops the choice lists`() {
        val fact = one.aircast.android.bridge.Fact("p", "MODE", "", "", "1", 1, listOf("A", "B"), listOf("0", "1"), 1, isBool = false, isString = false, readOnly = false)
        assertEquals(false, manualEntryFact(fact).isEnum)
    }

    @Test
    fun `value details read the control's description and default like the landing altitude dialog`() {
        val control = org.json.JSONObject("""{"path":"item.finalApproachAltitude","name":"FinalApproachAltitude","label":"Altitude","valueDetails":"Altitude to begin landing approach from.","units":"m","valueString":"40.0","value":40.0,"control":"number","readOnly":false,"minimumText":null,"maximumText":null,"defaultText":"40.0"}""")
        val fact = factFromControl(control)!!
        assertEquals("Altitude to begin landing approach from.", fact.valueDetails)
        assertEquals(listOf("Altitude to begin landing approach from.", "default 40.0 m"), valueDetailsNotes(fact))
    }
}
