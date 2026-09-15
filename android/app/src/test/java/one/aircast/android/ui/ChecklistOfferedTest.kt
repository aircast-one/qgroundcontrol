package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChecklistOfferedTest {

    @Test
    fun `an armed vehicle is past the checks rather than failing them`() {
        assertEquals(
            "QGC disables its own Checklist action on an armed vehicle - " +
                "PreFlightCheckListShowAction.qml gates enabled on !armed - and asking whether the " +
                "props are mounted is not a question for an aircraft already flying on them",
            "The checks are for before the flight",
            checklistOffered(armed = true),
        )
    }

    @Test
    fun `a disarmed vehicle is offered the checks as before`() {
        assertNull(checklistOffered(armed = false))
    }
}
