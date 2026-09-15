package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

private const val SERVED = """
{"class": "AltitudeModes", "context": "mission", "current": 2, "holdsAltitudeAboveTerrain": true,
 "modes": [
  {"raw": 1, "title": "Relative To Launch", "help": "Above the launch position.", "enabled": true, "current": false, "reason": ""},
  {"raw": 2, "title": "AMSL", "help": "Above mean sea level.", "enabled": true, "current": true, "reason": ""},
  {"raw": 4, "title": "Terrain Frame", "help": "Above terrain, held by the vehicle in flight.", "enabled": false, "current": false,
   "reason": "This vehicle does not hold an altitude above terrain."},
  {"raw": 0, "title": "Mixed Modes", "help": "Each item sets its own.", "enabled": true, "current": false, "reason": ""}
 ],
 "omitted": []}
"""

private const val EMPTY_PLAN = """
{"class": "AltitudeModes", "context": "mission", "current": 0, "holdsAltitudeAboveTerrain": true,
 "modes": [
  {"raw": 1, "title": "Relative To Launch", "help": "", "enabled": false, "current": false,
   "reason": "Add a mission item before choosing how its altitude is measured."},
  {"raw": 2, "title": "AMSL", "help": "", "enabled": false, "current": false,
   "reason": "Add a mission item before choosing how its altitude is measured."},
  {"raw": 4, "title": "Terrain Frame", "help": "", "enabled": false, "current": false,
   "reason": "Add a mission item before choosing how its altitude is measured."},
  {"raw": 0, "title": "Mixed Modes", "help": "", "enabled": true, "current": true, "reason": ""}
 ],
 "omitted": []}
"""

private const val ONE_OPTION = """
{"class": "AltitudeModes", "context": "mission", "current": 2, "holdsAltitudeAboveTerrain": false,
 "modes": [
  {"raw": 1, "title": "Relative To Launch", "help": "", "enabled": false, "current": false,
   "reason": "This vehicle reports no launch position."},
  {"raw": 2, "title": "AMSL", "help": "Above mean sea level.", "enabled": true, "current": true, "reason": ""},
  {"raw": 4, "title": "Terrain Frame", "help": "", "enabled": false, "current": false,
   "reason": "This vehicle does not hold an altitude above terrain."},
  {"raw": 0, "title": "Mixed Modes", "help": "", "enabled": true, "current": false, "reason": ""}
 ],
 "omitted": []}
"""

class AltitudeModesTest {
    @Test
    fun `one pickable mode is the one already set, so the control changes nothing`() {
        assertEquals(
            "the single enabled entry is the current mode - opening the control offers the operator " +
                "what they already have, and the two refusals are printed on the row either way",
            false,
            offersChoice(altitudeModesView(JSONObject(ONE_OPTION))),
        )
        assertEquals(1, choosable(altitudeModesView(JSONObject(ONE_OPTION))).count { it.enabled })
    }

    @Test
    fun `a picker holding one option promises a choice it cannot deliver`() {
        assertEquals(
            "on an empty plan the core enables only Mixed, which is never offered, so opening the " +
                "control shows entries that all refuse with the sentence the row already prints",
            false,
            offersChoice(altitudeModesView(JSONObject(EMPTY_PLAN))),
        )
    }

    @Test
    fun `a plan with real alternatives keeps the picker live`() {
        assertTrue(offersChoice(altitudeModesView(JSONObject(SERVED))))
    }

    @Test
    fun `the refused modes are still counted by the core's judgement, never by re-deriving why`() {
        val view = altitudeModesView(JSONObject(EMPTY_PLAN))
        assertEquals(3, choosable(view).size)
        assertEquals(
            "Add a mission item before choosing how its altitude is measured.",
            refusalFor(view, 1),
        )
    }

    @Test
    fun `mixed modes is a state the plan can be in, never a mode to pick`() {
        val picks = choosable(altitudeModesView(JSONObject(SERVED)))
        assertEquals(listOf(1, 2, 4), picks.map { it.raw })
        assertTrue("offering Mixed would ask the operator to set every item at once", picks.none { it.raw == ALT_MODE_MIXED })
    }

    @Test
    fun `a mode the vehicle cannot fly carries the core's reason rather than vanishing`() {
        assertEquals(
            "This vehicle does not hold an altitude above terrain.",
            refusalFor(altitudeModesView(JSONObject(SERVED)), 4),
        )
        assertNull("a mode that is offered has nothing to explain", refusalFor(altitudeModesView(JSONObject(SERVED)), 2))
    }

    @Test
    fun `the path carries the context and the current mode, which is what the core keys off`() {
        assertEquals("view.altitudeModes(mission,2)", altitudeModesPath(MISSION_CONTEXT, 2))
        assertEquals("plan.missionController.visualItems.3.altitudeMode", altitudeModePath(3))
    }

    @Test
    fun `anything that is not the altitude modes view is refused`() {
        assertNull(altitudeModesView(JSONObject("""{"class": "Radio"}""")))
        assertNull(altitudeModesView(null))
    }
}
