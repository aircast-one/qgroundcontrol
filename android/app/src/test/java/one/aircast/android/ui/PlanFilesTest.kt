package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Test

class SaveBlockedTest {
    private fun view(body: String) = org.json.JSONObject("""{"kind":"object","readiness":$body}""")

    @Test
    fun `a plan the core calls ready is not blocked`() {
        assertNull(saveBlockedReason(view("""{"state":0,"ready":true,"reason":""}""")))
    }

    @Test
    fun `the core's own sentence is shown, not this head's paraphrase of the same state`() {
        assertEquals(
            "Waiting for terrain heights before the plan can be saved or sent.",
            saveBlockedReason(view("""{"state":1,"ready":false,"reason":"Waiting for terrain heights before the plan can be saved or sent."}""")),
        )
        assertEquals(
            "An item is still being drawn, so the plan cannot be saved or sent.",
            saveBlockedReason(view("""{"state":2,"ready":false,"reason":"An item is still being drawn, so the plan cannot be saved or sent."}""")),
        )
    }

    @Test
    fun `no answer blocks the save rather than letting it through unchecked`() {
        assertEquals("The plan could not be checked for saving.", saveBlockedReason(null))
        assertEquals("The plan could not be checked for saving.", saveBlockedReason(org.json.JSONObject("""{"kind":"null"}""")))
    }

    @Test
    fun `a state the core has no sentence for still refuses rather than passing silently`() {
        assertEquals(
            "The plan could not be checked for saving.",
            saveBlockedReason(view("""{"state":9,"ready":false,"reason":""}""")),
        )
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

class PlanHistoryTest {

    @Test
    fun `a plan with nothing behind it offers neither`() {
        val fresh = planHistory(JSONObject("""{"kind":"object","canUndo":false,"canRedo":false}"""))

        assertEquals(PlanHistory(canUndo = false, canRedo = false), fresh)
    }

    @Test
    fun `an edited plan can be undone but not yet redone`() {
        val edited = planHistory(JSONObject("""{"kind":"object","canUndo":true,"canRedo":false}"""))

        assertEquals(PlanHistory(canUndo = true, canRedo = false), edited)
    }

    @Test
    fun `a view that never answered offers nothing, rather than enabled buttons that refuse`() {
        assertEquals(PlanHistory(canUndo = false, canRedo = false), planHistory(null))
        assertEquals(PlanHistory(canUndo = false, canRedo = false), planHistory(JSONObject("{}")))
    }
}
