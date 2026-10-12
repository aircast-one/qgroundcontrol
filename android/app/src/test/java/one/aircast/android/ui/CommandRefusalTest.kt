package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CommandRefusalTest {

    @Test
    fun `the aircraft's reply to a mode change is told once by the core, not repeated in the banner`() {
        assertTrue(isModeReply("No such mode 26"))
        assertTrue(isModeReply("Mode change to AUTO failed: requires position"))
        assertFalse(isModeReply("Arm: Mode not armable"))
        val reply = VehicleMessage(0, "12:00:00", "Warning", MessageSeverity.Warning, "No such mode 26")
        val fence = reply.copy(index = 1, text = "Fence breached")
        assertEquals(listOf(fence), bannerMessages(listOf(reply, fence)))
    }

    @Test
    fun `the aircraft's reply to an arm attempt is told once by the core, not repeated in the banner`() {
        assertTrue(isArmReply("Arm: Mode not armable"))
        assertTrue(isArmReply("Arming denied: Flight mode not allowed"))
        assertFalse(isArmReply("EKF3 IMU0 is using GPS"))
        val reply = VehicleMessage(0, "12:00:00", "Critical", MessageSeverity.Error, "Arm: Mode not armable")
        val crash = reply.copy(index = 1, text = "Crash: Disarming")
        assertEquals(listOf(crash), bannerMessages(listOf(reply, crash)))
    }

    @Test
    fun `sent is a brief acknowledgement, gone before a refusal could sit under it`() {
        assertEquals(3_000L, SENT_SHOWN_MS)
    }
}
