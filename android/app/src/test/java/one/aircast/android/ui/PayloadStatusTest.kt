package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PayloadStatusTest {
    private val at = 1_000_000L

    private fun state(action: String, outcome: String) = JSONObject(mapOf("action" to action, "outcome" to outcome, "atMs" to at))

    @Test
    fun `says nothing until a payload command has been sent`() {
        assertNull(payloadStatus(null, at))
        assertNull(payloadStatus(state("release", "lost"), at))
    }

    @Test
    fun `tells the pilot what the flight controller answered and how long ago`() {
        assertEquals(PayloadStatus("Drop sent, waiting for the aircraft", warning = false), payloadStatus(state("release", "sent"), at + 500))
        assertEquals(PayloadStatus("Dropped 12 s ago", warning = false), payloadStatus(state("release", "confirmed"), at + 12_400))
        assertEquals(PayloadStatus("Grabbed 1 min ago", warning = false), payloadStatus(state("grab", "confirmed"), at + 65_000))
        assertEquals(PayloadStatus("Drop refused by the flight controller 2 s ago", warning = true), payloadStatus(state("release", "refused"), at + 2_000))
    }

    @Test
    fun `a drop the aircraft never answered is a warning not a failure`() {
        assertEquals(PayloadStatus("Drop not confirmed 3 s ago. It may have dropped, check before dropping again.", warning = true), payloadStatus(state("release", "unanswered"), at + 3_000))
        assertEquals(PayloadStatus("Grab not confirmed 3 s ago. Check the gripper before trying again.", warning = true), payloadStatus(state("grab", "unanswered"), at + 3_000))
    }

    @Test
    fun `a basket names the bay in every line`() {
        val bay2 = JSONObject(mapOf("action" to "release", "bay" to 2, "outcome" to "confirmed", "atMs" to at))
        assertEquals(PayloadStatus("Bay 2 dropped 4 s ago", warning = false), payloadStatus(bay2, at + 4_000))
        val basket = Payload(3, listOf(2), 1)
        assertEquals(listOf("Slide to drop bay 1", "Bay 2 empty, slide to drop"), listOf(bayLabel(1, basket), bayLabel(2, basket)))
    }

    @Test
    fun `the payload carries what would stop it dropping and how to fix it`() {
        val view = JSONObject(
            """{"payload":{"bays":4,"empty":[1],"next":2,"label":"Drop 2","problems":[
                {"code":"staysOpen","text":"The gripper stays open after the first drop.","blocking":true,"fix":"Turn on auto-close"},
                {"code":"noGripper","text":"No gripper.","blocking":true,"fix":""}]}}""",
        )
        assertEquals(
            Payload(
                4,
                listOf(1),
                2,
                listOf(
                    PayloadProblem("staysOpen", "The gripper stays open after the first drop.", blocking = true, fix = "Turn on auto-close"),
                    PayloadProblem("noGripper", "No gripper.", blocking = true, fix = ""),
                ),
                "Drop 2",
            ),
            payloadOf(view),
        )
        assertEquals(SINGLE_GRIPPER, payloadOf(JSONObject("""{"payload":{"bays":1,"empty":[],"next":null}}""")))
    }

    @Test
    fun `time since is coarse and never negative`() {
        assertEquals(listOf("0 s ago", "59 s ago", "1 h ago"), listOf(agoText(at, at - 5_000), agoText(at, at + 59_999), agoText(at, at + 3_600_000)))
    }
}
