package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ControlRequestPromptTest {
    @Test
    fun `another station's request and the takeover countdown are read from the core`() {
        val asked = controlPrompt(JSONObject("""{"inControl":true,"incomingRequest":{"systemId":9,"timeoutMs":20000,"remainingMs":12500},"takeoverRevertMs":null}"""))
        assertEquals(IncomingControlRequest(9, 20000, 12500), asked.incoming)
        assertNull(asked.revertMs)
        assertEquals(13, secondsLeft(12500))
        assertEquals(4000L, controlPrompt(JSONObject("""{"inControl":true,"incomingRequest":null,"takeoverRevertMs":4000}""")).revertMs)
        assertNull("the countdown closes once the other station has taken control", controlPrompt(JSONObject("""{"inControl":false,"incomingRequest":null,"takeoverRevertMs":4000}""")).revertMs)
    }
}
