package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

private const val TWO_DEAD = """
{"class": "Radio", "connected": true, "channelCount": 8, "liveChannels": 6,
 "enoughChannels": true, "shortfall": "",
 "summary": "8 channels reported, 6 carrying a signal.",
 "sticks": [{"key": "yaw", "title": "Yaw", "value": 1173, "valueText": "1173",
             "fraction": 0.173, "mapped": true, "reversed": true}],
 "channels": [{"index": 6, "label": "7", "value": 0, "valueText": "—", "fraction": 0.0, "live": false},
              {"index": 0, "label": "1", "value": 1607, "valueText": "1607", "fraction": 0.607, "live": true}]}
"""

class RadioViewTest {
    @Test
    fun `a channel carrying no signal is a dash, never a pwm of zero`() {
        val dead = radioView(JSONObject(TWO_DEAD))!!.channels.first { !it.live }
        assertEquals("—", dead.valueText)
        assertFalse("a reading of 0 would claim the receiver measured the stick at its floor", dead.valueText == "0")
    }

    @Test
    fun `the summary counts the channels carrying a signal, not the channels reported`() {
        val view = radioView(JSONObject(TWO_DEAD))!!
        assertEquals(8, view.channelCount)
        assertEquals("8 channels reported, 6 carrying a signal.", view.summary)
        assertTrue("the count the header used to show cannot say two are dead", view.summary.contains("6 carrying"))
    }

    @Test
    fun `a reversed stick keeps that flag, which is the only thing marking it on the row`() {
        assertTrue(radioView(JSONObject(TWO_DEAD))!!.sticks.single().reversed)
    }

    @Test
    fun `anything that is not the radio view is refused rather than read as an empty radio`() {
        assertNull(radioView(JSONObject("""{"class": "Track", "points": []}""")))
        assertNull(radioView(null))
    }
}
