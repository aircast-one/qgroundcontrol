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
    fun `continue mission pops up too`() {
        assertEquals("continueMission", autoMissionPopup(emptySet(), mapOf("continueMission" to offer("continueMission", "ready")), enabled = true)?.id)
    }
}
