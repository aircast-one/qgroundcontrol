package one.aircast.android.ui

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
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

    @Test
    fun `the stored layout yields hidden widgets, offsets and the indicator order`() {
        val stored = mapOf("OverlayRigOffset-instrumentPanel" to "10.0,20.0", "OverlayRigHidden-orbit" to true, "FlyViewIndicatorOrder" to "gps,battery")
        assertEquals(mapOf("instrumentPanel" to (10f to 20f)), storedOffsets(stored))
        assertEquals(setOf("orbit"), hiddenKeys(stored))
        assertEquals(listOf("gps", "battery"), storedIndicatorOrder(stored))
    }

    @Test
    fun `portrait and landscape keep separate offsets`() {
        assertEquals("instrumentPanel", orientedKey("instrumentPanel", landscape = false))
        assertEquals("instrumentPanel@landscape", orientedKey("instrumentPanel", landscape = true))
    }

    @Test
    fun `a widget its own offset pushed off screen is pulled back, a scrolled or oversized one is left alone`() {
        val root = Size(1000f, 500f)
        assertEquals(-300f to -100f, onScreenCorrection(Rect(900f, 400f, 1300f, 600f), root, 400f to 300f))
        assertEquals(-100f to 0f, onScreenCorrection(Rect(900f, 0f, 1300f, 100f), root, 100f to 0f))
        assertEquals(40f to 0f, onScreenCorrection(Rect(-40f, 0f, 60f, 100f), root, -80f to 0f))
        assertEquals(0f to 0f, onScreenCorrection(Rect(1100f, 10f, 1300f, 200f), root, 0f to 0f))
        assertEquals(0f to 0f, onScreenCorrection(Rect(10f, 10f, 200f, 200f), root, 50f to 50f))
        assertEquals(0f to 0f, onScreenCorrection(Rect(20f, 10f, 1600f, 200f), root, 30f to 0f))
    }
}
