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
    fun `the default setting does not announce that a feature nobody asked for is not happening`() {
        assertNull(
            "followTarget defaults to 2, which Mode::from_setting maps to followMe - so a fresh " +
                "install with a connected vehicle that is not in Follow Me flight mode is the " +
                "ordinary resting state, and every operator was being told about it on the Fly " +
                "screen for a feature they never turned on",
            followMeLabel(
                followMeReading(
                    view(mode = "\"followMe\"", wouldSend = false, reason = "\"noVehicleInFollowMode\""),
                ),
            ),
        )
    }

    @Test
    fun `the core cannot report that reason under always, so the mode does not need testing`() {
        assertNull(
            "receives() is Mode::Always => true, so with a non-empty fleet reason() can never " +
                "return NoVehicleInFollowMode under always - a head that also checked the mode " +
                "would be guarding a state the core cannot produce, and macOS treats the token as " +
                "resting whatever the mode for the same reason",
            followMeLabel(
                followMeReading(
                    view(mode = "\"always\"", wouldSend = false, reason = "\"noVehicleInFollowMode\""),
                ),
            ),
        )
    }

    @Test
    fun `once a vehicle is in Follow Me mode every other trouble is still reported`() {
        assertEquals(
            "reason() returns NoVehicleInFollowMode before it looks at the fix at all, so any other " +
                "reason means a vehicle IS in Follow Me mode and the stream is failing anyway - " +
                "that is the operator's own request breaking, not a resting default",
            "Not following you \u2014 this phone has no position yet",
            followMeLabel(
                followMeReading(view(mode = "\"followMe\"", wouldSend = false, reason = "\"noFix\"")),
            ),
        )
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
            followMeReading(
                view(mode = "\"always\"", enabled = false, wouldSend = false, reason = "\"$token\""),
            ),
        )

        assertEquals("Not following you — this phone's position has stopped updating", stuck("fixStale"))
        assertEquals("Not following you — this phone's position is not valid", stuck("fixInvalid"))
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
            mode = "\"always\"",
            enabled = false,
            wouldSend = false,
            reason = "\"fixStale\"",
            vehicles = """[{"id":1,"following":false,"refusal":"notInFollowMode"}]""",
        )

        assertEquals(
            "Not following you — this phone's position has stopped updating",
            followMeLabel(followMeReading(asked)),
        )
    }

    @Test
    fun `no vehicle at all is nothing to say, however the setting reads`() {
        val alone = view(enabled = false, wouldSend = false, reason = "\"noVehicles\"", vehicles = "[]")

        assertNull(followMeLabel(followMeReading(alone)))
    }
}
