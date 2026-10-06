package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FlyStateTest {
    @Test
    fun `a link nobody is monitoring is not reported as having contact`() {
        val unknown = flyState(
            JSONObject("""{"kind":"object","class":"FlyState","connected":true,"contactLost":null}"""),
        )
        assertNull(
            "flystate.rs serves contactLost as an Option and says so: communicationLostEnabled " +
                "off means unknown, and unknown is not evidence of a loss. optBoolean flattens " +
                "JSON null to false, which turns \"nobody is watching this link\" into \"contact " +
                "is fine\" - the direction that reassures",
            unknown?.contactLost,
        )
        assertEquals(false, flyState(JSONObject("""{"kind":"object","class":"FlyState","contactLost":false}"""))?.contactLost)
        assertEquals(true, flyState(JSONObject("""{"kind":"object","class":"FlyState","contactLost":true}"""))?.contactLost)
    }

    private val inContact = JSONObject(
        """{"class":"FlyState","connected":true,"armed":false,"flying":false,"landing":false,
            "contactLost":false,"state":"disarmed","stateText":"Stabilize · Disarmed",
            "staleNotice":"","mode":"Stabilize"}""",
    )

    private val lost = JSONObject(
        """{"class":"FlyState","connected":true,"armed":false,"flying":false,"landing":false,
            "contactLost":true,"state":"contactLost","stateText":"Communication lost",
            "staleNotice":"No contact — these are the last values the vehicle sent.",
            "mode":"Stabilize"}""",
    )

    @Test
    fun `the sentence and the token come from the core, not from the head`() {
        val state = flyState(inContact)!!

        assertEquals("disarmed", state.state)
        assertEquals("Stabilize · Disarmed", state.stateText)
        assertEquals("Ready to fly", flyState(JSONObject("""{"kind":"object","class":"FlyState","connected":true,"stateText":"Ready to Fly"}"""))?.stateText)
    }

    @Test
    fun `a vehicle in contact carries no notice, so nothing dims`() {
        val state = flyState(inContact)!!

        assertEquals(false, state.contactLost)
        assertEquals("", state.staleNotice)
    }

    @Test
    fun `lost contact carries the notice and the head shows it verbatim`() {
        val state = flyState(lost)!!

        assertEquals(true, state.contactLost)
        assertEquals("Communication lost", state.stateText)
        assertEquals(
            "No contact — these are the last values the vehicle sent.",
            state.staleNotice,
        )
    }

    @Test
    fun `the notice uses the core's dash, not an ascii hyphen`() {
        assertTrue(flyState(lost)!!.staleNotice.contains("—"))
        assertFalse(flyState(lost)!!.staleNotice.contains(" - "))
    }

    @Test
    fun `a payload that is not the fly state yields nothing`() {
        assertNull(flyState(null))
        assertNull(flyState(JSONObject("{}")))
        assertNull(flyState(JSONObject("""{"class":"Calibration"}""")))
    }

    @Test
    fun `a payload using an invented name for the notice leaves it blank`() {
        val invented = JSONObject(
            """{"class":"FlyState","contactLost":true,"stale":"No contact","notice":"No contact"}""",
        )

        assertEquals("", flyState(invented)!!.staleNotice)
    }
}

class VehicleSubtitleTest {
    private fun state(
        connected: Boolean = true,
        contactLost: Boolean = false,
        stateText: String = "Disarmed",
        mode: String = "Stabilize",
    ) = FlyState(connected, false, contactLost, "disarmed", stateText, "", mode, false, "", null, null, null)

    @Test
    fun `the header keeps the flight mode the core reports separately`() {
        assertEquals("Stabilize · Disarmed", vehicleSubtitle(state()))
    }

    @Test
    fun `a lost link says so on its own, because the mode is no longer known`() {
        assertEquals(
            "Communication lost",
            vehicleSubtitle(state(contactLost = true, stateText = "Communication lost")),
        )
    }

    @Test
    fun `no vehicle says so rather than showing an empty line`() {
        assertEquals("No vehicle", vehicleSubtitle(null))
        assertEquals("No vehicle", vehicleSubtitle(state(connected = false)))
    }

    @Test
    fun `with no vehicle the header shows the link state like MainStatusIndicator`() {
        val offline = offlineMainStatus(JSONObject("""{"kind":"object","title":"Connection Failed","mainStatus":"Can't Connect"}"""))
        assertEquals("Can't connect", vehicleSubtitle(state(connected = false), offline))
        assertEquals("Connect a vehicle", vehicleSubtitle(null, offlineMainStatus(JSONObject("""{"mainStatus":"Connect a Vehicle"}"""))))
        assertEquals("a connected vehicle ignores the link state", "Stabilize · Disarmed", vehicleSubtitle(state(), offline))
        assertNull(offlineMainStatus(JSONObject("""{"mainStatus":""}""")))
    }

    @Test
    fun `a blank mode does not leave a dangling separator`() {
        assertEquals("Armed", vehicleSubtitle(state(stateText = "Armed", mode = "")))
    }
}

class FlyStateArmedTest {

    private fun view(armed: Boolean, connected: Boolean = true) = flyState(
        JSONObject(
            """{"kind":"object","class":"FlyState","connected":$connected,"armed":$armed,
               "contactLost":false,"state":"armed","stateText":"Armed","staleNotice":"",
               "mode":"Stabilize","rcSupported":false,"rcSignal":null,"rcSignalText":null}""",
        ),
    )

    @Test
    fun `armed comes from the view rather than a second read of the vehicle`() {
        assertTrue(view(armed = true)!!.armed)
        assertFalse(view(armed = false)!!.armed)
    }

    @Test
    fun `no vehicle is not an armed vehicle`() {
        assertFalse(flyState(JSONObject("""{"kind":"object","class":"FlyState","connected":false}"""))!!.armed)
    }

    @Test
    fun `the status sheet summary is the core's line`() {
        val state = flyState(JSONObject("""{"class":"FlyState","connected":true,"stateText":"Not Fully Ready","summaryDetail":"Mag turned off. Everything else reports normal."}"""))!!
        assertEquals("Mag turned off. Everything else reports normal.", state.summaryDetail)
        assertEquals("", flyState(JSONObject("""{"class":"FlyState","summaryDetail":null}"""))!!.summaryDetail)
    }
}

class FlyReadinessTest {
    private fun state(armed: Boolean = false, ready: Boolean = true, nominal: Boolean = true, fault: Boolean = false, flying: String = "disarmed") = FlyState(
        connected = true, armed = armed, contactLost = false, state = flying, stateText = if (ready) "Ready to fly" else "Not ready", staleNotice = "", mode = "Stabilize",
        rcSupported = false, rcSignalText = "", rcSignal = null, rcOverride = null, telemetry = null,
        summaryDetail = "2 checks need attention before arming.", nominal = nominal, fault = fault, readyToFly = ready,
    )

    @Test
    fun aVehicleThatIsNotReadyIsNotShownGreen() {
        assertEquals(ChipTone.Neutral, chipTone(state(ready = false), lost = false))
        assertEquals(ChipTone.Success, chipTone(state(), lost = false))
        assertEquals(ChipTone.Warning, chipTone(state(nominal = false), lost = false))
        assertEquals(ChipTone.Error, chipTone(state(fault = true), lost = false))
        assertEquals(ChipTone.Success, chipTone(state(armed = true, ready = false), lost = false))
    }

    @Test
    fun theReadinessWarningSaysWhyAndOnlyBeforeArming() {
        assertEquals("Not ready. 2 checks need attention before arming.", readinessWarning(state(ready = false)))
        assertNull(readinessWarning(state()))
        assertNull(readinessWarning(state(armed = true, ready = false)))
        assertEquals("Not ready. Vehicle setup is not complete.", readinessWarning(state(ready = false).copy(summaryDetail = ALL_CHECKS_PASSED)))
    }

    @Test
    fun aReadinessIssueBlocksOnlyWhenTheVehicleCannotArm() {
        assertEquals(Readiness("Not ready. 2 checks need attention before arming.", blocks = false), guidedReadiness(state(ready = false)))
        assertEquals(
            Readiness("Not ready. 2 checks need attention before arming. $READINESS_BLOCKED", blocks = true),
            guidedReadiness(state(ready = false).copy(canArm = false)),
        )
        assertNull(guidedReadiness(state()))
    }

    @Test
    fun disarmingSaysWhetherTheVehicleFlew() {
        assertEquals("Landed and disarmed", disarmNotice(wasArmed = true, flewWhileArmed = true, armedNow = false))
        assertEquals("Disarmed", disarmNotice(wasArmed = true, flewWhileArmed = false, armedNow = false))
        assertNull(disarmNotice(wasArmed = false, flewWhileArmed = false, armedNow = false))
    }

    @Test
    fun theMapLeadsWhileArmedWithNoVideoSource() {
        assertEquals(FlyView.Map, flyViewShown(FlyView.Video, armed = true, noVideoSource = true))
        assertEquals(FlyView.Video, flyViewShown(FlyView.Video, armed = false, noVideoSource = true))
        assertEquals(FlyView.Video, flyViewShown(FlyView.Video, armed = true, noVideoSource = false))
    }
}
