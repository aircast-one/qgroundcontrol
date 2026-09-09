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
