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

    @Test
    fun `a dragged widget stops at the screen edges`() {
        assertEquals(30f to -40f, clampedDrag(100f, 40f, 200f, 140f, 1000f, 2000f, 30f, -80f))
        assertEquals(-100f to 1860f, clampedDrag(100f, 40f, 200f, 140f, 1000f, 2000f, -500f, 5000f))
        assertEquals(800f to 0f, clampedDrag(100f, 40f, 200f, 140f, 1000f, 2000f, 900f, 0f))
    }

    @Test
    fun `stored offsets are read back per widget and malformed ones are dropped`() {
        val stored = mapOf("OverlayRigOffset-instrumentPanel" to "12.5,-30.0", "OverlayRigOffset-orbit" to "bad", "OverlayRigHidden-traffic" to true)
        assertEquals(mapOf("instrumentPanel" to (12.5f to -30f)), storedOffsets(stored))
    }
}
