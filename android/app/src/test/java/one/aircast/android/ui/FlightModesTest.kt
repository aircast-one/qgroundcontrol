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
         "modes":[
           {"name":"Stabilize","summary":"You fly it; it keeps itself level.","current":true,
            "advanced":false,"needsConfirm":false},
           {"name":"Loiter","summary":"Holds position.","current":false,
            "advanced":false,"needsConfirm":false},
           {"name":"Acro","summary":"Rate mode, no self-levelling.","current":false,
            "advanced":true,"needsConfirm":false}]}
    """

    @Test
    fun `every mode the vehicle offers is listed in its order`() {
        assertEquals(listOf("Stabilize", "Loiter", "Acro"), flightModesView(JSONObject(served))!!.all.map { it.name })
    }

    @Test
    fun `the current mode is marked so the list can tick it`() {
        val modes = flightModesView(JSONObject(served))!!

        assertEquals("Stabilize", modes.current)
        assertTrue(modes.all.first { it.name == "Stabilize" }.current)
    }

    @Test
    fun `the confirm flag is read from the mode, not guessed from its name`() {
        val flying = flightModesView(
            JSONObject("""{"available":true,"canSet":true,"current":"Loiter",
                "modes":[
                  {"name":"RTL","summary":"Flies home.","current":false,
                   "advanced":false,"needsConfirm":true},
                  {"name":"Loiter","summary":"Holds position.","current":true,
                   "advanced":false,"needsConfirm":false}]}"""),
        )!!

        assertTrue(flying.all.first { it.name == "RTL" }.needsConfirm)
        assertFalse(flying.all.first { it.name == "Loiter" }.needsConfirm)
    }

    @Test
    fun `the popover lists the core's quick modes with their cautions`() {
        val modes = flightModesView(
            JSONObject("""{"available":true,"canSet":true,"current":"Stabilize",
                "pinnedSetting":"settings.flightModeSettings.apmPinnedFlightModesFixedWing",
                "quick":[
                  {"name":"Cruise","quick":true,"caution":""},
                  {"name":"Loiter","quick":true,"caution":"Needs GPS"}],
                "modes":[
                  {"name":"Stabilize","current":true},
                  {"name":"Cruise","quick":true},
                  {"name":"Loiter","quick":true,"caution":"Needs GPS"}]}"""),
        )!!

        assertEquals(listOf("Cruise", "Loiter"), modes.quick.map { it.name })
        assertEquals("Needs GPS", modes.quick.last().caution)
        assertEquals(listOf(false, true, true), modes.all.map { it.quick })
        assertEquals("settings.flightModeSettings.apmPinnedFlightModesFixedWing", modes.pinnedSetting)
    }

    @Test
    fun `the core's reason for refusing every mode change is carried to the menu`() {
        val refused = flightModesView(JSONObject("""{"available":true,"canSet":false,"cannotSetNotice":"This vehicle does not accept a flight mode change from here."}"""))!!
        assertEquals("This vehicle does not accept a flight mode change from here.", refused.cannotSetNotice)
        assertEquals("", flightModesView(JSONObject("""{"available":true,"canSet":true}"""))!!.cannotSetNotice)
    }

    @Test
    fun `a vehicle reporting no modes offers no picker`() {
        assertNull(flightModesView(null))
        assertNull(flightModesView(JSONObject("""{"available":false}""")))
    }
}

class UnknownModeTest {
    @Test
    fun `the core's unknown-mode line is carried to the menu`() {
        val view = org.json.JSONObject("""{"available":true,"current":"Custom 7","unknownModeNotice":"The vehicle is in Custom 7, which this version of the app doesn't know. Choose a mode below to change it."}""")
        org.junit.Assert.assertTrue(flightModesView(view)!!.unknownModeNotice.startsWith("The vehicle is in Custom 7"))
        org.junit.Assert.assertEquals("", flightModesView(org.json.JSONObject("""{"available":true}"""))!!.unknownModeNotice)
    }
}

class ModeHeadingTest {

    private fun modes(current: String, summary: String) = flightModesView(
        JSONObject(
            """{"kind":"object","class":"FlightModes","available":true,"canSet":true,"current":$current,
               "currentSummary":$summary}""",
        ),
    )

    @Test
    fun `the heading names the mode and what it does`() {
        assertEquals(
            "Stabilize — You fly it by hand, it only levels itself",
            modeHeading(modes("\"Stabilize\"", "\"You fly it by hand, it only levels itself\"")),
        )
    }

    @Test
    fun `a mode the core has no sentence for gets no heading rather than a dangling dash`() {
        assertNull(modeHeading(modes("\"Stabilize\"", "\"\"")))
        assertNull(modeHeading(modes("\"\"", "\"Something\"")))
        assertNull(modeHeading(null))
    }

    @Test
    fun `pinning starts from the quick list on show and adds or drops one mode`() {
        assertEquals("Cruise,FBW A,Acro", pinsAfter(listOf("Cruise", "FBW A"), "Acro", pin = true))
        assertEquals("FBW A", pinsAfter(listOf("Cruise", "FBW A"), "Cruise", pin = false))
        assertEquals("", pinsAfter(listOf("Cruise"), "Cruise", pin = false))
        assertEquals("Cruise", pinsAfter(listOf("Cruise"), "Cruise", pin = true))
    }

    @org.junit.Test
    fun `each mode in the menu carries an icon like the Penpot mode picker`() {
        org.junit.Assert.assertEquals(
            listOf(one.aircast.android.R.drawable.ic_gamepad, one.aircast.android.R.drawable.ic_height, one.aircast.android.R.drawable.ic_my_location, one.aircast.android.R.drawable.ic_route, one.aircast.android.R.drawable.ic_home, one.aircast.android.R.drawable.ic_flight),
            listOf("Stabilize", "Altitude Hold", "Position Hold", "Auto", "Smart RTL", "Circle").map(::flightModeIcon),
        )
    }

    @Test
    fun `a divider starts the return modes like the QGC mode sections`() {
        val modes = flightModesView(
            JSONObject("""{"available":true,"modes":[
                {"name":"Loiter","section":"normal"},{"name":"Guided"},
                {"name":"RTL","section":"return"},{"name":"Land","section":"return"}]}"""),
        )!!.all

        assertEquals(listOf(false, false, true, false), modes.indices.map { startsSection(modes, it) })
    }
}
