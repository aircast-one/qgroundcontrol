package one.aircast.android.bridge

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class FactParseTest {

    @Test
    fun `a json null string field reads as empty, not as the word null`() {
        val json = JSONObject(
            """{"name":"FLTMODE_CH","shortDescription":null,"units":null,"valueString":"5.000",
                "minString":null,"maxString":null,"defaultValueString":null}""",
        )

        val fact = Qgc.fact("settings.x", json)

        assertEquals("", fact.defaultValueString)
        assertEquals("", fact.minString)
        assertEquals("", fact.maxString)
        assertEquals("", fact.description)
        assertEquals("", fact.units)
        assertEquals("5.000", fact.valueString)
    }

    @Test
    fun `a fact with real strings keeps them`() {
        val json = JSONObject(
            """{"name":"RTL_ALT","shortDescription":"Return altitude","units":"cm",
                "valueString":"1500","minString":"0","maxString":"8000","defaultValueString":"1500"}""",
        )

        val fact = Qgc.fact("", json)

        assertEquals("RTL_ALT", fact.path)
        assertEquals("Return altitude", fact.description)
        assertEquals("cm", fact.units)
        assertEquals("1500", fact.defaultValueString)
    }
}

class TruthyTest {
    @Test
    fun `a real boolean is read as itself`() {
        assertEquals(true, truthy(true))
        assertEquals(false, truthy(false))
    }

    @Test
    fun `a property declared int but meaning bool still reads true`() {
        assertEquals(true, truthy(1))
        assertEquals(false, truthy(0))
    }

    @Test
    fun `a string encoding is accepted either way round`() {
        assertEquals(true, truthy("true"))
        assertEquals(true, truthy("1"))
        assertEquals(false, truthy("0"))
        assertEquals(false, truthy("false"))
    }

    @Test
    fun `an absent or unreadable value is not true`() {
        assertEquals(false, truthy(null))
        assertEquals(false, truthy("banana"))
    }
}
