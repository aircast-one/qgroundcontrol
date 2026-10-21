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

    @Test
    fun `auto intercept says what it chases, why it waits, how it ended and what to do next`() {
        val shown = { json: String -> autodropShown(JSONObject(json)) }
        assertNull("idle has nothing to say", autodropShown(null))
        assertEquals(
            AutodropShown("Locked on", false, listOf(AutodropAction.DropOnIt)),
            autodropShown(JSONObject("""{"state":"idle"}"""), locked = true),
        )
        assertEquals(
            AutodropShown("Chasing the car · 45 m · 5 m/s · release in ~3 s", false, listOf(AutodropAction.Stop)),
            shown("""{"state":"waiting","progress":{"target":"car","distanceMeters":45.2,"speedMps":5.0,"moving":true,"releaseInSeconds":3.2,"holding":null,"secondsLeft":71}}"""),
        )
        assertEquals(
            "Chasing the car · 60 m · 0 m/s · release in ~9 s · not moving — is this the right one?",
            shown("""{"state":"waiting","progress":{"target":"car","distanceMeters":60,"speedMps":0.1,"moving":false,"releaseInSeconds":9,"secondsLeft":71}}""")?.text,
        )
        assertEquals(
            "Holding: the car is turning · 52 s left",
            shown("""{"state":"waiting","progress":{"target":"car","holding":"turning","secondsLeft":52}}""")?.text,
        )
        assertEquals(
            AutodropShown("Lost the car — tap it to pick it up again · 40 s left", true, listOf(AutodropAction.Stop)),
            shown("""{"state":"waiting","progress":{"target":"car","holding":"searching","searching":true,"secondsLeft":40}}"""),
        )
        assertEquals(
            AutodropShown("Hit the car · 3 left", false, listOf(AutodropAction.DropAgain)),
            shown("""{"state":"fired","impact":"seen","estimatedMissMeters":0.7,"estimatedOnTarget":true,"progress":{"target":"car"},"payload":{"bays":4,"bay":2,"left":3}}"""),
        )
        assertEquals(
            AutodropShown("Missed by about 3 m · no bottles left", true, emptyList()),
            shown("""{"state":"fired","impact":"seen","estimatedMissMeters":2.6,"estimatedOnTarget":false,"progress":{"target":"car"},"payload":{"bays":1,"bay":null,"left":0}}"""),
        )
        assertEquals(
            AutodropShown("Couldn't line up a safe drop in 90 s — holding position", true, listOf(AutodropAction.TryAgain, AutodropAction.PickAnother)),
            shown("""{"state":"gaveUp","reason":"no release window opened within the time limit","timedOut":true}"""),
        )
    }

    @Test
    fun `the auto intercept confirmation names the target and the bay`() {
        assertEquals(
            "Chase the car you're tracking (62 m, moving) and drop bay 2 when it lines up · 3 loaded. The drone steers and drops by itself.",
            autodropConfirm(
                JSONObject("""{"kind":"value","value":{"label":"car","source":"tracked","speedMps":5.0,"distanceMeters":61.6}}"""),
                JSONObject("""{"payload":{"bays":4,"bay":2,"left":3}}"""),
            ),
        )
        assertEquals(
            "Chase the marked point (120 m) and drop the payload when it lines up · 1 loaded. The drone steers and drops by itself.",
            autodropConfirm(
                JSONObject("""{"kind":"value","value":{"label":"marked point","source":"manual","speedMps":0,"distanceMeters":120}}"""),
                JSONObject("""{"payload":{"bays":1,"bay":0,"left":1}}"""),
            ),
        )
        assertEquals(
            "Nothing to chase yet — track a target with the camera or mark one on the map.",
            autodropConfirm(JSONObject("""{"kind":"value","value":null}"""), null),
        )
    }
}
