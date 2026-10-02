package one.aircast.android.ui

import org.junit.Assert.assertEquals
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
    fun `the status drawer's arm controls follow MainStatusIndicator`() {
        assertEquals(ArmControls("Slide to Arm", true, false, false, false), armControls(state(), forceOpen = false))
        assertEquals(ArmControls("Slide to Disarm", true, false, false, false), armControls(state(armed = true), forceOpen = false))
        assertEquals(ArmControls("Slide to Arm", true, true, false, false), armControls(state(nominal = false), forceOpen = false))
        assertEquals(ArmControls("Slide to Arm", false, false, true, false), armControls(state(nominal = false, fault = true, canArm = false), forceOpen = false))
        assertEquals(ArmControls("Slide to Arm", false, false, false, true), armControls(state(canArm = false), forceOpen = true))
    }

    @Test
    fun `a slide the deck cannot act on says why instead of closing silently`() {
        val blocked = GuidedOffer(id = "arm", title = "Arm", offer = "disabled", reason = "Complete the preflight checklist first", prompt = "", destructive = false, carriesValue = false)
        assertEquals("Complete the preflight checklist first", deckRequestRefusal(blocked))
        assertEquals(ARM_UNAVAILABLE, deckRequestRefusal(null))
    }
}
