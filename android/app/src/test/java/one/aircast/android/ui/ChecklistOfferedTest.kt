package one.aircast.android.ui

import org.json.JSONObject
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

    @Test
    fun `an operator who turned the checklist off in Settings is not offered it`() {
        assertEquals(false, preflightOffered(JSONObject("{\"offered\":false}")))
        assertEquals(true, preflightOffered(JSONObject("{\"offered\":true}")))
    }

    @Test
    fun `a core too old to serve the field keeps offering the checks`() {
        assertEquals(
            "offered arrived in 98cd88175, so a head running against an older core sees no key at " +
                "all - defaulting that to false would silently withdraw the checklist from every " +
                "operator rather than from the ones who asked for it to go",
            true,
            preflightOffered(JSONObject("{\"airframe\":\"Quadrotor\"}")),
        )
        assertEquals(true, preflightOffered(null))
    }

    @Test
    fun `being past the checks and never having wanted them are different answers`() {
        assertEquals(
            "armed dims a row that is still drawn, because the state will pass; offered removes the " +
                "row, because the operator withdrew it - one answer for both would either nag " +
                "someone who opted out or leave them no reason for a dimmed row",
            "The checks are for before the flight",
            checklistOffered(armed = true),
        )
        assertEquals(true, preflightOffered(JSONObject("{\"offered\":true}")))
    }
}
