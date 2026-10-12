package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ModeOutcomeTest {
    private val earlier = ModeAck(serial = 4, accepted = true)

    @Test
    fun `a mode change settles, or stops waiting on a refusal or silence, which the core words`() {
        assertEquals(ModeOutcome.Settled, modeOutcome(earlier, earlier, reached = true, elapsedMs = 100))
        assertEquals(ModeOutcome.Pending, modeOutcome(earlier, earlier, reached = false, elapsedMs = 100))
        assertEquals(ModeOutcome.Unsettled, modeOutcome(earlier, ModeAck(5, false), reached = false, elapsedMs = 100))
        assertEquals("an old refusal is not this one", ModeOutcome.Pending, modeOutcome(ModeAck(5, false), ModeAck(5, false), reached = false, elapsedMs = 100))
        assertEquals(ModeOutcome.Unsettled, modeOutcome(null, null, reached = false, elapsedMs = MODE_REPLY_MS))
    }

    @Test
    fun `the core's ack reads into the head`() {
        assertEquals(ModeAck(2, false), modeAck(JSONObject("""{"modeAck":{"serial":2,"accepted":false,"wording":"refused for now"}}""")))
        assertNull(modeAck(JSONObject("""{"modeAck":null}""")))
    }
}
