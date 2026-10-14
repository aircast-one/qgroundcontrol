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

        assertFalse(offers.getValue("arm").ready)
        assertTrue(offers.getValue("takeoff").ready)
        assertFalse(offers.getValue("land").shown)
        assertEquals("Vehicle is not ready to arm.", offers.getValue("arm").reason)
        assertEquals("The vehicle will climb.", offers.getValue("takeoff").prompt)
        assertTrue(offers.getValue("arm").destructive)
    }

    @Test
    fun `a disarm the core marks as unconfirmed goes out on the tap, everything else still asks`() {
        val offers = guidedOffers(JSONObject("""{"actions":[
            {"id":"disarm","title":"Disarm","offer":"ready","prompt":"Disarm the vehicle","destructive":false,"confirm":false,"carriesValue":false},
            {"id":"land","title":"Land","offer":"ready","prompt":"Land","destructive":false,"confirm":true,"carriesValue":false},
            {"id":"rtl","title":"Return","offer":"ready","prompt":"Return","destructive":false,"carriesValue":false}]}"""))
        val action = { id: String -> GuidedAction(name = id, confirm = "", destructive = false, offerId = id, run = {}) }

        assertFalse(needsConfirm(action("disarm"), offers))
        assertTrue(needsConfirm(action("land"), offers))
        assertTrue("an offer that does not say asks first", needsConfirm(action("rtl"), offers))
        assertTrue("an action with no offer behind it asks first", needsConfirm(action("gone"), offers))
    }

    @Test
    fun `no view is no offers rather than a crash`() {
        assertEquals(emptyMap<String, GuidedOffer>(), guidedOffers(null))
        assertEquals(emptyMap<String, GuidedOffer>(), guidedOffers(JSONObject("{}")))
    }
}
