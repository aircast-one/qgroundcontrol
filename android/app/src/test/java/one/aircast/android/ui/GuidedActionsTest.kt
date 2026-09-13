package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GuidedActionsTest {

    private val served = """
        {"connected":true,"forwardFlight":false,"missionActive":false,"actions":[
          {"id":"arm","title":"Arm","offer":"blocked","reason":"Vehicle is not ready to arm.",
           "prompt":"Arming spins the propellers.","destructive":true,"carriesValue":false},
          {"id":"takeoff","title":"Takeoff","offer":"ready","reason":"",
           "prompt":"The vehicle will climb.","destructive":false,"carriesValue":true},
          {"id":"land","title":"Land","offer":"hidden","reason":"",
           "prompt":"","destructive":false,"carriesValue":false}]}
    """

    @Test
    fun `an offer carries its state, reason and prompt`() {
        val offers = guidedOffers(JSONObject(served))

        assertTrue(offers.getValue("arm").blocked)
        assertTrue(offers.getValue("takeoff").ready)
        assertFalse(offers.getValue("land").shown)
        assertEquals("Vehicle is not ready to arm.", offers.getValue("arm").reason)
        assertEquals("The vehicle will climb.", offers.getValue("takeoff").prompt)
        assertTrue(offers.getValue("arm").destructive)
    }

    @Test
    fun `a blocked action explains itself and a ready one does not`() {
        val offers = guidedOffers(JSONObject(served))

        assertEquals("Vehicle is not ready to arm.", blockedReasonFor(offers["arm"]))
        assertNull(blockedReasonFor(offers["takeoff"]))
        assertNull(blockedReasonFor(null))
    }

    @Test
    fun `a blocked action with no reason still says something`() {
        val offers = guidedOffers(
            JSONObject("""{"actions":[{"id":"rtl","title":"RTL","offer":"blocked","reason":""}]}"""),
        )

        assertEquals("The vehicle will not accept this yet.", blockedReasonFor(offers["rtl"]))
    }

    @Test
    fun `no view is no offers rather than a crash`() {
        assertEquals(emptyMap<String, GuidedOffer>(), guidedOffers(null))
        assertEquals(emptyMap<String, GuidedOffer>(), guidedOffers(JSONObject("{}")))
    }
}
