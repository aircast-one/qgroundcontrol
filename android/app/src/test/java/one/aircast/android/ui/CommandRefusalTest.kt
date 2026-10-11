package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CommandRefusalTest {

    @Test
    fun `a late confirmation withdraws only its own sentence`() {
        val late = "Loiter was not confirmed by the aircraft."
        assertNull(withdrawn(late, late))
        assertEquals(
            "a newer refusal shown since belongs to another command and stays",
            "Altitude Hold was not confirmed by the aircraft.",
            withdrawn("Altitude Hold was not confirmed by the aircraft.", late),
        )
        assertNull(withdrawn(null, late))
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
