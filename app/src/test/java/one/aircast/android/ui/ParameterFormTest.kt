package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
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
    fun `a parameter the vehicle does not have is skipped`() {
        assertNull(factFromParameter("NOPE", JSONObject("""{"kind":"value","value":null}""")))
        assertNull(factFromParameter("NOPE", JSONObject("""{"kind":"null"}""")))
        assertNull(factFromParameter("NOPE", JSONObject("{}")))
    }

    @Test
    fun `safety sections are chosen by firmware`() {
        val apm = setupSectionsFor("Safety", isPx4 = false)!!
        val px4 = setupSectionsFor("Safety", isPx4 = true)!!
        assertEquals(true, apm.any { it.names.contains("FS_THR_ENABLE") })
        assertEquals(true, px4.any { it.names.contains("NAV_RCL_ACT") })
        assertEquals(false, apm.any { it.names.contains("NAV_RCL_ACT") })
    }

    @Test
    fun `components without a native form report none`() {
        assertNull(setupSectionsFor("Sensors", isPx4 = false))
        assertNull(setupSectionsFor("Radio", isPx4 = true))
    }
}
