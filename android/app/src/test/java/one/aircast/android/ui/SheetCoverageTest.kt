package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SheetCoverageTest {

    private val everyServedAction = listOf(
        "arm", "takeoff", "startMission", "continueMission", "resumeMission",
        "cancelRoi", "pause", "changeAltitude", "changeSpeed", "landAbort",
        "land", "rtl", "disarm", "grab", "release", "emergencyStop",
        "vtolTransitionToFixedWing", "vtolTransitionToMultiRotor", "forceArm",
    )

    private fun allReady(): Map<String, GuidedOffer> {
        val actions = everyServedAction.joinToString(",") { id ->
            """{"id":"$id","title":"$id","offer":"ready","reason":"","prompt":"",
                "destructive":false,"carriesValue":false}"""
        }
        return guidedOffers(JSONObject("""{"actions":[$actions]}"""))
    }

    @Test
    fun `the bar and the sheet never offer the same action`() {
        assertEquals(
            "an action in both places is two buttons for one command, and the operator " +
                "cannot tell which one the vehicle heard",
            emptySet<String>(),
            BAR_ACTIONS intersect SHEET_ACTIONS,
        )
    }

    @Test
    fun `every action the core can serve is reachable somewhere`() {
        val unreachable = everyServedAction
            .filterNot { it in BAR_ACTIONS || it in SHEET_ACTIONS || it == EMERGENCY_STOP }

        assertEquals(
            "a served action drawn in neither place is one the operator can never send; " +
                "the VTOL transitions were exactly that until this commit",
            emptyList<String>(),
            unreachable,
        )
    }

    @Test
    fun `the sheet holds every ready action that is not on the bar`() {
        val shown = moreActions(allReady()).map { it.id }.toSet()

        assertTrue(
            "the transitions are how a VTOL changes between hover and forward flight, and " +
                "this head drew neither",
            "vtolTransitionToFixedWing" in shown && "vtolTransitionToMultiRotor" in shown,
        )
        assertTrue("forceArm" in shown)
        assertTrue(
            "nothing on the bar may also be in the sheet",
            shown.none { it in BAR_ACTIONS },
        )
        assertTrue("the pinned stop is never in the sheet", EMERGENCY_STOP !in shown)
    }

    @Test
    fun `every action the sheet offers can actually be sent`() {
        moreActions(allReady()).forEach { offer ->
            if (offer.id != PAUSE) {
                assertNotNull(
                    "${offer.id} is drawn in the sheet but this head has no way to send it",
                    guidedCommand(offer.id, 3),
                )
            }
        }
    }
}
