package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class MoreActionsTest {

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
