package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AutoMissionPopupTest {
    private fun offer(id: String, offer: String) = GuidedOffer(id, id, offer, "", "", destructive = false, carriesValue = false)

    @Test
    fun `start mission pops up the moment it becomes available and not again`() {
        val ready = mapOf("startMission" to offer("startMission", "ready"))
        assertEquals("startMission", autoMissionPopup(emptySet(), ready, enabled = true)?.id)
        assertNull(autoMissionPopup(setOf("startMission"), ready, enabled = true))
        assertNull(autoMissionPopup(emptySet(), ready, enabled = false))
        assertNull(autoMissionPopup(emptySet(), mapOf("startMission" to offer("startMission", "blocked")), enabled = true))
    }

    @Test
    fun `land abort pops up on a fixed-wing approach even with mission popups off`() {
        val approach = mapOf("landAbort" to offer("landAbort", "ready"))
        assertEquals("GuidedActionsController's onShowLandAbortChanged confirms it whatever enableAutomaticMissionPopups says", "landAbort", autoMissionPopup(emptySet(), approach, enabled = false)?.id)
        assertNull(autoMissionPopup(setOf("landAbort"), approach, enabled = false))
        assertEquals("confirmAction(actionLandAbort) replaces whatever confirm is open", true, popupReplacesOpenConfirm("landAbort"))
        assertEquals(false, popupReplacesOpenConfirm("startMission"))
    }

    @Test
    fun `continue mission pops up too`() {
        assertEquals("continueMission", autoMissionPopup(emptySet(), mapOf("continueMission" to offer("continueMission", "ready")), enabled = true)?.id)
    }

    @Test
    fun `a press on the video closes a start or continue mission prompt but never the land abort`() {
        assertEquals(true, dismissesMissionPrompt("startMission"))
        assertEquals(true, dismissesMissionPrompt("continueMission"))
        assertEquals(false, dismissesMissionPrompt("landAbort"))
        assertEquals(false, dismissesMissionPrompt("rtl"))
        assertEquals(false, dismissesMissionPrompt(null))
    }

    @Test
    fun `the click menu opens over a mission prompt it closes, but never over another guided confirm`() {
        assertEquals(true, clickMenuAllowed(openDecision(null, confirming = false, panelOpen = false)))
        assertEquals(true, clickMenuAllowed(openDecision("startMission", confirming = true, panelOpen = false)))
        assertEquals(true, clickMenuAllowed(openDecision("continueMission", confirming = true, panelOpen = false)))
        assertEquals(false, clickMenuAllowed(openDecision("rtl", confirming = true, panelOpen = false)))
        assertEquals(false, clickMenuAllowed(openDecision("landAbort", confirming = true, panelOpen = false)))
        assertEquals(false, clickMenuAllowed(openDecision(null, confirming = true, panelOpen = false)))
        assertEquals(false, clickMenuAllowed(openDecision(null, confirming = false, panelOpen = true)))
    }
}
