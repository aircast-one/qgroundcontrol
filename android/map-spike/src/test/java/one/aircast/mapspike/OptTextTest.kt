package one.aircast.mapspike

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class OptTextTest {

    @Test
    fun `a field the core withheld reads as absent, not as the word null`() {
        val json = JSONObject("""{"altitudeText":null,"distanceText":"449 m"}""")

        assertEquals("", json.optText("altitudeText"))
        assertEquals("449 m", json.optText("distanceText"))
    }

    @Test
    fun `a key that was never there is absent too`() {
        assertEquals("", JSONObject("{}").optText("nope"))
    }

    @Test
    fun `a withheld element of a list reads as absent`() {
        val list = JSONArray("""["a",null,"c"]""")

        assertEquals("a", list.optText(0))
        assertEquals("", list.optText(1))
        assertEquals("c", list.optText(2))
    }
}
