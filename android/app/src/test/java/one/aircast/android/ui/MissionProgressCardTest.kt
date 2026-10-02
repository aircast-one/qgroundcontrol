package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MissionProgressCardTest {
    @Test
    fun `the card reads the core's progress and offers the next waypoint until the last`() {
        val view = JSONObject("""{"shown":true,"current":3,"last":6,"fraction":0.5,"distanceToNext":"84","distanceUnits":"m","canSkip":true,"skipTo":4}""")
        val progress = missionProgress(view)!!
        assertEquals("To waypoint 3 of 6 · 84 m", missionProgressLine(progress))
        assertEquals(4, progress.skipTo)
        assertNull(missionProgress(view.put("shown", false)))
        assertNull(missionProgress(JSONObject("""{"shown":true,"current":6,"last":6,"canSkip":false,"skipTo":6}"""))!!.skipTo)
    }
}
