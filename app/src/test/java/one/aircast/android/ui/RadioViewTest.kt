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

private const val MID_CALIBRATION = """
{"class": "Radio", "connected": true, "channelCount": 8, "liveChannels": 8,
 "enoughChannels": true, "shortfall": "", "summary": "8 channels reported, 8 carrying a signal.",
 "calibrating": true, "statusText": "Move the Throttle stick all the way up and hold it there...",
 "nextText": "Next", "nextEnabled": true, "cancelEnabled": true, "skipEnabled": true,
 "sticks": [], "channels": []}
"""

class RadioViewTest {
    @Test
    fun `a calibration in progress carries the step text and the buttons the core enables`() {
        val cal = radioView(JSONObject(MID_CALIBRATION))!!.calibration
        assertTrue(cal.running)
        assertEquals("Next", cal.nextText)
        assertTrue(cal.cancelEnabled)
        assertTrue("a step the operator cannot perform has to be skippable", cal.skipEnabled)
        assertTrue(cal.statusText.startsWith("Move the Throttle stick"))
    }

    @Test
    fun `an idle radio offers the start label and nothing to cancel`() {
        val idle = radioView(JSONObject(TWO_DEAD))!!.calibration
        assertFalse(idle.running)
        assertFalse("nothing is running, so cancel would cancel nothing", idle.cancelEnabled)
    }

    @Test
    fun `the action path names the controller the core reads its state from`() {
        assertEquals("radioCal.nextButtonClicked", radioCalAction("nextButtonClicked"))
        assertEquals("radioCal.cancelButtonClicked", radioCalAction("cancelButtonClicked"))
    }

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

class CalibrationStepTest {
    @Test
    fun `the line telling the operator to click is dropped, because the button is right there`() {
        val served = "Lower the Throttle stick all the way down.\n\n" +
            "Reset all transmitter trims to center.\n\nClick Next to continue"
        assertEquals(
            "Lower the Throttle stick all the way down.\n\nReset all transmitter trims to center.",
            calibrationStep(served),
        )
    }

    @Test
    fun `a step that does not end in an instruction to click is left alone`() {
        val served = "Move the Throttle stick all the way up and hold it there..."
        assertEquals(served, calibrationStep(served))
    }

    @Test
    fun `nothing served is nothing shown`() {
        assertEquals("", calibrationStep(""))
        assertEquals("", calibrationStep("   \n  "))
    }
}
