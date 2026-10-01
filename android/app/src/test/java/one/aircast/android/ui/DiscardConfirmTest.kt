package one.aircast.android.ui

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.json.JSONObject
import org.junit.Test

class DiscardConfirmTest {

    @Test
    fun `dirty and syncing come off the plan view the tab already reads`() {
        val idle = JSONObject("""{"kind":"object","class":"PlanStatus","dirty":false,"sync":{"state":"ready"}}""")
        val busy = JSONObject("""{"kind":"object","class":"PlanStatus","dirty":true,"sync":{"state":"busy"}}""")
        assertFalse(planIsDirty(idle))
        assertTrue(planIsDirty(busy))
        assertFalse(planIsSyncing(idle))
        assertTrue("plan.rs serves busy for a sync in flight; the head compared against a syncing it never sends", planIsSyncing(busy))
        assertFalse(planIsSyncing(JSONObject("""{"kind":"object","sync":{"state":"offline"}}""")))
        assertTrue(planContainsItems(JSONObject("""{"kind":"object","containsItems":true}""")))
        assertFalse(planContainsItems(null))
    }

    @Test
    fun `no plan view is not a clean plan and not a finished sync`() {
        assertFalse(planIsDirty(null))
        assertFalse(planIsSyncing(null))
        assertFalse(planIsSyncing(JSONObject("""{"kind":"object","class":"PlanStatus"}""")))
    }
}
