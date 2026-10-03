package one.aircast.android.ui

import one.aircast.mapspike.altitudeModesView
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MissionAltitudeFrameTest {
    @Test
    fun `the mission frame is read from the plan view`() {
        assertEquals(1, globalAltitudeFrame(JSONObject("""{"globalAltitudeFrame":1}""")))
        assertNull(globalAltitudeFrame(JSONObject("""{"globalAltitudeFrame":null}""")))
    }

    @Test
    fun `mixed stays pickable for the plan once items exist, like MissionSettingsEditor's AltModeMenu`() {
        val view = altitudeModesView(JSONObject(ITEMS_ADDED))
        assertEquals(listOf(1, 2, 0), missionFramePicks(view).map { it.raw })
        assertTrue(missionFrameChoice(view))
    }

    @Test
    fun `only the current mode enabled leaves nothing to pick`() {
        assertFalse(missionFrameChoice(altitudeModesView(JSONObject(MIXED_ONLY))))
    }
}

private const val ITEMS_ADDED = """
{"class": "AltitudeModes", "context": "mission", "current": 1, "modes": [
  {"raw": 1, "title": "Relative To Launch", "enabled": true, "current": true},
  {"raw": 2, "title": "AMSL", "enabled": false, "current": false, "reason": "locked"},
  {"raw": 0, "title": "Mixed Modes", "enabled": true, "current": false}
 ], "omitted": []}
"""

private const val MIXED_ONLY = """
{"class": "AltitudeModes", "context": "mission", "current": 0, "modes": [
  {"raw": 1, "title": "Relative To Launch", "enabled": false, "current": false},
  {"raw": 0, "title": "Mixed Modes", "enabled": true, "current": true}
 ], "omitted": []}
"""
