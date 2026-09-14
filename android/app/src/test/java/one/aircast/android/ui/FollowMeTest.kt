package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FollowMeTest {

    private fun view(
        mode: String = "\"followMe\"",
        enabled: Boolean = true,
        wouldSend: Boolean = true,
        reason: String = "null",
        vehicles: String = """[{"id":1,"following":true,"refusal":""}]""",
    ) = JSONObject(
        """{"kind":"object","class":"FollowMe","mode":$mode,"enabled":$enabled,
           "wouldSend":$wouldSend,"reason":$reason,"vehicles":$vehicles,"count":1}""",
    )

    @Test
    fun `another view is not a follow reading`() {
        assertNull(followMeReading(null))
        assertNull(followMeReading(JSONObject("""{"kind":"object","class":"Orbit"}""")))
    }

    @Test
    fun `a vehicle taking the stream says so plainly`() {
        assertEquals("Following you", followMeLabel(followMeReading(view())))
    }

    @Test
    fun `a switched-off feature shows nothing at all, rather than a complaint`() {
        val off = view(mode = "\"never\"", enabled = false, wouldSend = false, reason = "\"modeNever\"")

        assertNull(followMeLabel(followMeReading(off)))
    }

    @Test
    fun `switched on and not sending names the thing standing in the way`() {
        fun stuck(token: String) = followMeLabel(
            followMeReading(view(enabled = false, wouldSend = false, reason = "\"$token\"")),
        )

        assertEquals("Not following you — this phone's position has stopped updating", stuck("fixStale"))
        assertEquals("Not following you — no vehicle is in Follow Me mode", stuck("noVehicleInFollowMode"))
        assertEquals("Not following you — the vehicle refused the position", stuck("allVehiclesRefused"))
    }

    @Test
    fun `a token this head has never seen still produces a sentence`() {
        assertEquals(
            "Not following you — the position is not being sent",
            followMeLabel(followMeReading(view(enabled = false, wouldSend = false, reason = "\"somethingNew\""))),
        )
    }

    @Test
    fun `more than one follower is worth counting, one is not`() {
        val pair = view(
            vehicles = """[{"id":1,"following":true},{"id":2,"following":true}]""",
        )

        assertEquals("Following you · 2 vehicles", followMeLabel(followMeReading(pair)))
    }

    @Test
    fun `enabled is the timer running, not the operator asking, so the chip cannot key on it`() {
        val asked = view(
            enabled = false,
            wouldSend = false,
            reason = "\"noVehicleInFollowMode\"",
            vehicles = """[{"id":1,"following":false,"refusal":"notInFollowMode"}]""",
        )

        assertEquals(
            "Not following you — no vehicle is in Follow Me mode",
            followMeLabel(followMeReading(asked)),
        )
    }

    @Test
    fun `no vehicle at all is nothing to say, however the setting reads`() {
        val alone = view(enabled = false, wouldSend = false, reason = "\"noVehicles\"", vehicles = "[]")

        assertNull(followMeLabel(followMeReading(alone)))
    }
}
