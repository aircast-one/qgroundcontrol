package one.aircast.android.ui

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DiscardConfirmTest {

    @Test
    fun `a dirty plan with items is confirmed before it is discarded`() {
        assertTrue(discardNeedsConfirming(dirty = true, containsItems = true))
    }

    @Test
    fun `a clean plan is discarded without asking`() {
        assertFalse(discardNeedsConfirming(dirty = false, containsItems = true))
        assertFalse(discardNeedsConfirming(dirty = false, containsItems = false))
    }

    @Test
    fun `an empty plan is not confirmed, because dirty does not mean edited while connected`() {
        assertFalse(
            "PlanMasterController clears dirty in two places and BOTH are behind offline(), so " +
                "with a vehicle connected neither saving nor starting a new plan clears it and the " +
                "flag means 'not synced to the vehicle'. Observed on the handset 2026-09-10: both " +
                "discard confirmations fired on an EMPTY plan immediately after a successful save. " +
                "containsItems is the mitigation, not an oversight",
            discardNeedsConfirming(dirty = true, containsItems = false),
        )
    }

    @Test
    fun `the cost of that mitigation, so it is a decision and not a surprise`() {
        assertFalse(
            "QGC prompts on dirty ALONE (PlanView.qml:689 and :1443) and " +
                "MissionController::containsItems is visualItems.count > 1, so a mission carrying " +
                "only its settings item is 'empty'. Move the planned home and start a new plan: " +
                "QGC asks, this head does not. That is the price of not prompting spuriously on " +
                "every connected empty plan, and it is only removable once dirty means one thing",
            discardNeedsConfirming(dirty = true, containsItems = false),
        )
    }
}
