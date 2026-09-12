package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.json.JSONArray
import org.json.JSONObject
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
    private fun view(status: String) = org.json.JSONObject("""{"kind":"object","status":"$status"}""")

    @Test
    fun `the core spells the status, including whether changes are unsaved or unsent`() {
        assertEquals("ridge.plan \u00b7 not uploaded", planStatusText(view("ridge.plan \u00b7 not uploaded")))
        assertEquals("ridge.plan \u00b7 unsaved changes", planStatusText(view("ridge.plan \u00b7 unsaved changes")))
    }

    @Test
    fun `no answer shows nothing, so a silent core is visible rather than papered over`() {
        assertEquals("", planStatusText(null))
        assertEquals("", planStatusText(org.json.JSONObject("""{"kind":"null"}""")))
    }
}

class PlanActionsTest {
    private fun actions(
        open: Boolean = true,
        save: Boolean = true,
        exportKml: Boolean = true,
        newPlan: Boolean = true,
        clearMission: Boolean = true,
    ) = planActions(
        org.json.JSONObject(
            """{"kind":"object","actions":{"open":$open,"save":$save,"exportKml":$exportKml,
               "newPlan":$newPlan,"clearMission":$clearMission}}""",
        ),
    )

    @Test
    fun `each action is the core's answer rather than this head's arithmetic`() {
        assertEquals(false, actions(save = false).save)
        assertEquals(true, actions(save = false).open)
        assertEquals(false, actions(exportKml = false).exportKml)
    }

    @Test
    fun `clearMission is the core's name for clearing the vehicle, which is what this button does`() {
        assertEquals(false, actions(clearMission = false).clearFromVehicle)
        assertEquals(true, actions(clearMission = true).clearFromVehicle)
    }

    @Test
    fun `no answer offers nothing rather than guessing what is allowed`() {
        val none = planActions(null)
        assertEquals(false, none.open)
        assertEquals(false, none.save)
        assertEquals(false, none.newPlan)
    }
}
