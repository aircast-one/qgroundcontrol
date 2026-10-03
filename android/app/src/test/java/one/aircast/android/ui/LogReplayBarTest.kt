package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

class LogReplayBarTest {
    @Test
    fun the_bar_reads_the_core_replay_state() {
        assertNull(logReplay(JSONObject("""{"available":false}""")))
        val replay = logReplay(JSONObject("""{"available":true,"shown":true,"loaded":true,"playing":false,"percent":12.5,"playheadTime":"01m:15s","totalTime":"10m:00s","speedIndex":4,"speeds":["0.1","0.25","0.5","1x","2x","5x","10x"],"canLoad":false,"loadRefusal":"x","error":""}"""))!!
        assertEquals("2x", replay.speeds[replay.speedIndex])
        assertEquals(12.5f, replay.percent)
        assertFalse(replay.playing)
    }

    @Test
    fun the_bar_reads_its_position_and_speeds_like_the_design() {
        val replay = logReplay(JSONObject("""{"available":true,"shown":true,"playheadTime":"01m:15s","totalTime":"10m:00s","speeds":["0.25","2x"]}"""))!!
        assertEquals("01m:15s of 10m:00s", replayProgress(replay))
        assertEquals("", replayProgress(replay.copy(playheadTime = "", totalTime = "")))
        assertEquals(listOf("0.25×", "2×"), replay.speeds.map(::speedLabel))
    }
}
