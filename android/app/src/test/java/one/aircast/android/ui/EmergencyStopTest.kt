package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class EmergencyStopTest {

    private fun offers(vararg entries: Pair<String, String>): Map<String, GuidedOffer> {
        val actions = entries.joinToString(",") { (id, offer) ->
            """{"id":"$id","title":"$id","offer":"$offer","reason":"","prompt":"",
                "destructive":false,"carriesValue":false}"""
        }
        return guidedOffers(JSONObject("""{"actions":[$actions]}"""))
    }

    @Test
    fun `the emergency stop is never in the overflow sheet`() {
        assertFalse(
            "cutting the motors is the one action that must not be two taps behind a menu; " +
                "it is pinned, so the sheet must never also offer it",
            EMERGENCY_STOP in SHEET_ACTIONS,
        )
    }

    @Test
    fun `a ready emergency stop appears in no sheet listing`() {
        val extras = moreActions(offers(EMERGENCY_STOP to "ready", "grab" to "ready"))

        assertTrue(extras.none { it.id == EMERGENCY_STOP })
        assertEquals(listOf("grab"), extras.map { it.id })
    }

    @Test
    fun `the pinned button takes the offer the core serves`() {
        assertNotNull(emergencyStopOffer(offers(EMERGENCY_STOP to "ready")))
        assertNotNull(
            "a blocked stop is still drawn, with its reason - hiding it would remove the " +
                "control at the moment an operator reaches for it",
            emergencyStopOffer(offers(EMERGENCY_STOP to "blocked")),
        )
    }

    @Test
    fun `a hidden emergency stop draws nothing`() {
        assertNull(emergencyStopOffer(offers(EMERGENCY_STOP to "hidden")))
        assertNull(emergencyStopOffer(offers("grab" to "ready")))
    }

    @Test
    fun `the confirmation is always destructive, whatever the core says`() {
        val offer = offers(EMERGENCY_STOP to "ready").getValue(EMERGENCY_STOP)

        assertFalse("the fixture deliberately says false", offer.destructive)
        assertTrue(
            "this one cuts the motors in flight; the confirmation must read as destructive " +
                "even if the served flag does not say so",
            emergencyStopAction(offer).destructive,
        )
    }
}
