package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class OverlayLayoutTest {
    @Test
    fun `hidden widgets are read from the stored rig keys only`() {
        val stored = mapOf("OverlayRigHidden-orbit" to true, "OverlayRigHidden-traffic" to false, "other" to true)
        assertEquals(setOf("orbit"), hiddenKeys(stored))
    }

    @Test
    fun `hiding and showing toggles one key`() {
        assertEquals(setOf("a", "b"), withHidden(setOf("a"), "b", true))
        assertEquals(emptySet<String>(), withHidden(setOf("a"), "a", false))
    }

    @Test
    fun `reset layout arms on the first tap and resets on the second, like resetPill`() {
        assertEquals(ResetTap(reset = false, armed = true), resetTap(armed = false))
        assertEquals(ResetTap(reset = true, armed = false), resetTap(armed = true))
        assertEquals("Reset layout", resetPillText(false))
        assertEquals("Tap again to reset", resetPillText(true))
        assertEquals(4000L, RESET_ARM_MILLIS)
    }
}
