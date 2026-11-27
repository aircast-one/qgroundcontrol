package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class ChipOpensTest {

    private fun opens(lost: Boolean = false, choosing: Boolean = false, disconnected: Boolean = false, blocked: Boolean = false, narrow: Boolean = false) =
        chipOpens(lost, choosing, disconnected, blocked, narrow)

    @Test
    fun `the mode name opens the flight modes, also while something blocks arming beside its note`() {
        assertEquals(ChipOpens.Modes, opens())
        assertEquals("the note beside the name opens the reason", ChipOpens.Modes, opens(blocked = true))
        assertEquals(ChipOpens.Modes, opens(narrow = true))
    }

    @Test
    fun `in portrait, where no note is drawn, a blocked mode name opens the arming messages`() {
        assertEquals(ChipOpens.Why, opens(blocked = true, narrow = true))
    }

    @Test
    fun `a lost link, a choice of vehicles and no vehicle come before the modes and the reason`() {
        assertEquals(ChipOpens.Lost, opens(lost = true, choosing = true, disconnected = true, blocked = true, narrow = true))
        assertEquals(ChipOpens.Picking, opens(choosing = true, disconnected = true, blocked = true, narrow = true))
        assertEquals(ChipOpens.Offline, opens(disconnected = true, blocked = true, narrow = true))
    }
}
