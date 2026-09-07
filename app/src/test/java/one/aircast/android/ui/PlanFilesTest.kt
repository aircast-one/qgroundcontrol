package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class SaveGuardTest {
    @Test
    fun `a ready plan saves`() {
        assertNull(saveBlockedReason(READY_FOR_SAVE))
    }

    @Test
    fun `a plan still fetching terrain is refused, because its altitudes are wrong`() {
        assertEquals(
            "Waiting on terrain data. Saving now would store wrong altitudes.",
            saveBlockedReason(NOT_READY_TERRAIN),
        )
    }

    @Test
    fun `a plan with incomplete items is refused`() {
        assertEquals(
            "Some items still need a position or a value.",
            saveBlockedReason(NOT_READY_DATA),
        )
    }

    @Test
    fun `an unreadable readiness check blocks the save rather than allowing it`() {
        assertEquals("The plan could not be checked for saving.", saveBlockedReason(null))
    }

    @Test
    fun `a state this build does not know about blocks the save`() {
        assertEquals("The plan could not be checked for saving.", saveBlockedReason(7))
    }
}

class PlanStatusTest {
    @Test
    fun `an untouched plan is new, not unsaved`() {
        assertEquals("New plan", planStatusText(null, dirty = false))
    }

    @Test
    fun `an edited plan with no file says so`() {
        assertEquals("Unsaved plan", planStatusText(null, dirty = true))
    }

    @Test
    fun `a saved plan is named`() {
        assertEquals("mission.plan", planStatusText("mission.plan", dirty = false))
    }

    @Test
    fun `edits after a save are called out next to the name`() {
        assertEquals("mission.plan · unsaved changes", planStatusText("mission.plan", dirty = true))
    }
}
