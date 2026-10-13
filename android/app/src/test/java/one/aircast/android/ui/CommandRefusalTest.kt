package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class CommandRefusalTest {

    @Test
    fun `a reply the core already worded as a banner is not repeated in the message banner`() {
        val reply = VehicleMessage(0, "12:00:00", "Warning", MessageSeverity.Warning, "No such mode 26", toldElsewhere = true)
        val fence = VehicleMessage(1, "12:00:00", "Warning", MessageSeverity.Warning, "Fence breached")
        assertEquals(listOf(fence), bannerMessages(listOf(reply, fence)))
        val read = vehicleMessages(JSONObject("""{"order":"oldestFirst","items":[{"index":0,"text":"Arm: Mode not armable","level":"error","toldElsewhere":true},{"index":1,"text":"Crash: Disarming","level":"error"}]}"""))
        assertEquals("the core's flag travels into the head", listOf("Crash: Disarming"), bannerMessages(read).map { it.text })
    }

    @Test
    fun `sent is a brief acknowledgement, gone before a refusal could sit under it`() {
        assertEquals(3_000L, SENT_SHOWN_MS)
    }
}
