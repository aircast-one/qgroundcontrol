package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FlightModesTest {

    private val served = """
        {"available":true,"canSet":true,"current":"Stabilize",
         "currentSummary":"You fly it; it keeps itself level.",
         "everyday":[
           {"name":"Stabilize","summary":"You fly it; it keeps itself level.","current":true,
            "advanced":false,"needsConfirm":false},
           {"name":"Loiter","summary":"Holds position.","current":false,
            "advanced":false,"needsConfirm":false}],
         "folded":[
           {"name":"Acro","summary":"Rate mode, no self-levelling.","current":false,
            "advanced":true,"needsConfirm":false}]}
    """

    @Test
    fun `the everyday modes are separate from the folded ones`() {
        val modes = flightModesView(JSONObject(served))!!

        assertEquals(listOf("Stabilize", "Loiter"), modes.everyday.map { it.name })
        assertEquals(listOf("Acro"), modes.folded.map { it.name })
    }

    @Test
    fun `the current mode is marked so the list can tick it`() {
        val modes = flightModesView(JSONObject(served))!!

        assertEquals("Stabilize", modes.current)
        assertTrue(modes.everyday.first { it.name == "Stabilize" }.current)
    }

    @Test
    fun `the confirm flag is read from the mode, not guessed from its name`() {
        val flying = flightModesView(
            JSONObject("""{"available":true,"canSet":true,"current":"Loiter",
                "everyday":[
                  {"name":"RTL","summary":"Flies home.","current":false,
                   "advanced":false,"needsConfirm":true},
                  {"name":"Loiter","summary":"Holds position.","current":true,
                   "advanced":false,"needsConfirm":false}],
                "folded":[]}"""),
        )!!

        assertTrue(flying.everyday.first { it.name == "RTL" }.needsConfirm)
        assertFalse(flying.everyday.first { it.name == "Loiter" }.needsConfirm)
        assertFalse(flightModesView(JSONObject(served))!!.folded.first { it.name == "Acro" }.needsConfirm)
    }

    @Test
    fun `a vehicle reporting no modes offers no picker`() {
        assertNull(flightModesView(null))
        assertNull(flightModesView(JSONObject("""{"available":false}""")))
    }
}
