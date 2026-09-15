package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class MoreActionsTest {

    @Test
    fun `an ROI lock the vehicle is holding can be released from this head`() {
        assertEquals(
            "the core offers CancelRoi only on roi_supported && roi_active && flying, so the sheet " +
                "showing it means a lock IS in force - and with roiActive drawn nowhere either, " +
                "leaving it out left the camera pinned to a point with no indication and no escape",
            listOf("cancelRoi"),
            moreActions(offers("cancelRoi" to "ready")).map { it.id },
        )
        assertNotNull(guidedCommand("cancelRoi"))
    }

    @Test
    fun `resuming a mission carries the waypoint the core holds, never one the head counted`() {
        assertNotNull(guidedCommand("resumeMission", resumeFrom = 5))
        assertEquals(listOf("resumeMission"), moreActions(offers("resumeMission" to "ready")).map { it.id })
    }

    @Test
    fun `without the waypoint there is no resume command to run`() {
        assertNull(
            "can_resume() requires resume_from_sequence > 0 and the field is served on that same " +
                "predicate, so a ready offer always carries the number - this is the unreachable " +
                "half, refusing to send rather than sending a resume to waypoint zero",
            guidedCommand("resumeMission", resumeFrom = null),
        )
    }

    @Test
    fun `the waypoint is read from the core, and a withheld one is not a zero`() {
        assertEquals(5, resumeFromSequence(JSONObject("""{"resumeFromSequence":5}""")))
        assertNull(resumeFromSequence(JSONObject("""{"resumeFromSequence":null}""")))
        assertNull(resumeFromSequence(JSONObject("""{}""")))
        assertNull(
            "guided.rs:274 withholds a non-positive sequence, so this guard is unreachable from " +
                "today's core - it is kept because a resume sent to waypoint zero would fly the " +
                "plan again from the start, which is the one thing resume exists not to do",
            resumeFromSequence(JSONObject("""{"resumeFromSequence":0}""")),
        )
        assertNull(resumeFromSequence(null))
    }


    private fun offers(vararg entries: Pair<String, String>): Map<String, GuidedOffer> {
        val actions = entries.joinToString(",") { (id, offer) ->
            """{"id":"$id","title":"$id","offer":"$offer","reason":"","prompt":"",
                "destructive":false,"carriesValue":false}"""
        }
        return guidedOffers(JSONObject("""{"actions":[$actions]}"""))
    }

    @Test
    fun `the sheet carries what the row does not`() {
        val extra = moreActions(
            offers(
                "arm" to "ready",
                "takeoff" to "ready",
                "startMission" to "ready",
                "pause" to "blocked",
                "emergencyStop" to "ready",
            ),
        )

        assertEquals(listOf("startMission", "pause", "emergencyStop"), extra.map { it.id })
    }

    @Test
    fun `an action the core hides is not offered`() {
        val extra = moreActions(offers("grab" to "hidden", "release" to "ready"))

        assertEquals(listOf("release"), extra.map { it.id })
    }

    @Test
    fun `an action this head cannot send is not offered`() {
        assertEquals(emptyList<String>(), moreActions(offers("orbit" to "ready")).map { it.id })
    }

    @Test
    fun `every action the sheet shows can be sent, except pause which asks for a height`() {
        val ids = moreActions(
            offers(
                "startMission" to "ready", "continueMission" to "ready", "landAbort" to "ready",
                "grab" to "ready", "release" to "ready", "emergencyStop" to "ready", "pause" to "ready",
            ),
        ).map { it.id }

        ids.filterNot { it == PAUSE }.forEach { assertNotNull(it, guidedCommand(it)) }
        assertNull(guidedCommand(PAUSE))
    }
}

class PrimaryBlockedReasonTest {
    private fun offer(id: String, offer: String, reason: String = "") = GuidedOffer(
        id = id, title = id, offer = offer, reason = reason, prompt = "",
        destructive = false, carriesValue = false,
    )

    @Test
    fun `a ready row explains nothing`() {
        assertNull(primaryBlockedReason(mapOf("arm" to offer("arm", "ready"))))
    }

    @Test
    fun `a blocked action explains itself`() {
        assertEquals(
            "Preflight checks are failing",
            primaryBlockedReason(mapOf("arm" to offer("arm", "blocked", "Preflight checks are failing"))),
        )
    }

    @Test
    fun `a blocked action with no reason still says something`() {
        assertEquals(
            "The vehicle will not accept this yet.",
            primaryBlockedReason(mapOf("takeoff" to offer("takeoff", "blocked"))),
        )
    }

    @Test
    fun `a hidden action is not explained`() {
        assertNull(primaryBlockedReason(mapOf("rtl" to offer("rtl", "hidden", "no vehicle"))))
    }

    @Test
    fun `arm outranks a later blocked action`() {
        assertEquals(
            "arm reason",
            primaryBlockedReason(
                mapOf(
                    "takeoff" to offer("takeoff", "blocked", "takeoff reason"),
                    "arm" to offer("arm", "blocked", "arm reason"),
                ),
            ),
        )
    }
}
