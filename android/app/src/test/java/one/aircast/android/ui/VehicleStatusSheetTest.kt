package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class VehicleStatusSheetTest {
    @Test
    fun `the status sheet lists faulty sensors until asked for the rest`() {
        val sensors = listOf(SensorHealth("Gyro", "unhealthy", "Error"), SensorHealth("Baro", "healthy", "Normal"))
        assertEquals(listOf("Gyro"), shownSensors(sensors, showAll = false).map { it.name })
        assertEquals(listOf("Gyro", "Baro"), shownSensors(sensors, showAll = true).map { it.name })
    }

    private fun state(armed: Boolean = false, nominal: Boolean = true, fault: Boolean = false, canArm: Boolean = true) = FlyState(
        connected = true, armed = armed, contactLost = false, state = "", stateText = "", staleNotice = "", mode = "",
        rcSupported = false, rcSignalText = "", rcSignal = null, rcOverride = null, telemetry = null,
        nominal = nominal, fault = fault, canArm = canArm,
    )

    @Test
    fun `the messages card starts collapsed and toggles like the MainStatusIndicator disclosure`() {
        assertEquals("Show messages", messagesToggleText(shown = false))
        assertEquals("Hide messages", messagesToggleText(shown = true))
    }

    @Test
    fun `the status drawer's arm controls follow MainStatusIndicator`() {
        assertEquals(ArmControls("Slide to arm", true, false, false, false), armControls(state(), forceOpen = false))
        assertEquals(ArmControls("Slide to disarm", true, false, false, false), armControls(state(armed = true), forceOpen = false))
        assertEquals(ArmControls("Slide to arm", true, true, false, false), armControls(state(nominal = false), forceOpen = false))
        assertEquals(ArmControls("Slide to arm", false, false, true, false), armControls(state(nominal = false, fault = true, canArm = false), forceOpen = false))
        assertEquals(ArmControls("Slide to arm", false, false, false, true), armControls(state(canArm = false), forceOpen = true))
    }

    @Test
    fun `a slide the deck cannot act on says why instead of closing silently`() {
        val blocked = GuidedOffer(id = "arm", title = "Arm", offer = "disabled", reason = "Complete the preflight checklist first", prompt = "", destructive = false, carriesValue = false)
        assertEquals("Complete the preflight checklist first", deckRequestRefusal(blocked))
        assertEquals(ARM_UNAVAILABLE, deckRequestRefusal(null))
    }

    @Test
    fun `a check with a description toggles open and shut, one without stays shut, like the Overall Status delegate`() {
        val detailed = ArmingCheck("GPS not ready", "Wait for a fix", "error")
        val bare = ArmingCheck("Battery low", "", "warning")
        assertEquals(setOf(0), expandedAfterTap(emptySet(), 0, detailed))
        assertEquals(emptySet<Int>(), expandedAfterTap(setOf(0), 0, detailed))
        assertEquals(emptySet<Int>(), expandedAfterTap(emptySet(), 1, bare))
    }

    @Test
    fun `a status line that names a payload problem carries the one-tap fix the payload panel offers for it`() {
        val open = "The gripper stays open after the first drop, so the other 3 drops release nothing. Turn on auto-close so it closes after each drop."
        val view = { problem: String ->
            flyState(JSONObject("""{"class":"FlyState","connected":true,"stateText":"Not Fully Ready","summaryDetail":"$open","summaryProblem":$problem}"""))
        }

        assertEquals(
            PayloadProblem("staysOpen", open, blocking = true, fix = "Turn on auto-close"),
            statusSummary(view("""{"code":"staysOpen","text":"$open","blocking":true,"fix":"Turn on auto-close"}"""))?.problem,
        )
        assertEquals(StatusSummaryLines("Not fully ready", open, null), statusSummary(view("null")))
        assertNull(statusSummary(flyState(JSONObject("""{"class":"FlyState","summaryDetail":""}"""))))
    }
}
