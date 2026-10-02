package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ModeOutcomeTest {
    private val earlier = ModeAck(serial = 4, accepted = true, wording = "")

    @Test
    fun `a mode change settles, is refused by the ack, or times out like FlightModeIndicator`() {
        assertEquals(ModeOutcome.Settled, modeOutcome("Loiter", earlier, earlier, reached = true, elapsedMs = 100))
        assertEquals(ModeOutcome.Pending, modeOutcome("Loiter", earlier, earlier, reached = false, elapsedMs = 100))
        assertEquals(ModeOutcome.Rejected("Loiter denied"), modeOutcome("Loiter", earlier, ModeAck(5, false, "denied"), reached = false, elapsedMs = 100))
        assertEquals("an old refusal is not this one", ModeOutcome.Pending, modeOutcome("Loiter", ModeAck(5, false, "denied"), ModeAck(5, false, "denied"), reached = false, elapsedMs = 100))
        assertEquals(ModeOutcome.Rejected("Loiter: no reply"), modeOutcome("Loiter", null, null, reached = false, elapsedMs = MODE_REPLY_MS))
    }

    @Test
    fun `the core's ack reads into the head`() {
        assertEquals(ModeAck(2, false, "refused for now"), modeAck(JSONObject("""{"modeAck":{"serial":2,"accepted":false,"wording":"refused for now"}}""")))
        assertNull(modeAck(JSONObject("""{"modeAck":null}""")))
    }
}
