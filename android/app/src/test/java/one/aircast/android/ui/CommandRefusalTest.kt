package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CommandRefusalTest {

    @Test
    fun `a command the aircraft reached says nothing at all`() {
        assertNull(
            "this is the direction that matters: every arm, disarm and mode change that works " +
                "must leave no message, or the pilot learns to ignore the one that means something",
            commandRefusal("Arm", confirmed = true),
        )
        assertNull(commandRefusal("Loiter", confirmed = true))
    }

    @Test
    fun `an unconfirmed command says it was not confirmed, not that it failed`() {
        assertEquals(
            "attemptCommand gives up after COMMAND_SETTLE_MS and cannot tell a refused command " +
                "from a slow one, so the sentence claims only what it knows",
            "Arm was not confirmed by the aircraft.",
            commandRefusal("Arm", confirmed = false),
        )
        assertEquals("Disarm was not confirmed by the aircraft.", commandRefusal("Disarm", confirmed = false))
    }

    @Test
    fun `a flight mode name reads as the subject of the sentence`() {
        assertEquals(
            "the mode picker passes the mode's display name, so the sentence has to read with an " +
                "arbitrary one in front of it - including the numeric names an unknown mode gets",
            "Altitude Hold was not confirmed by the aircraft.",
            commandRefusal("Altitude Hold", confirmed = false),
        )
        assertEquals("Mode 65536 was not confirmed by the aircraft.", commandRefusal("Mode 65536", confirmed = false))
    }

    @Test
    fun `the settle window is long enough to be a wait and short enough to be an answer`() {
        assertEquals(4000L, COMMAND_SETTLE_MS)
    }
}
