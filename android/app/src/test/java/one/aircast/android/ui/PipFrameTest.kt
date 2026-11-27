package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.pinch
import androidx.compose.ui.test.swipe
import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class PipFrameTest {
    @get:Rule
    val rule = createComposeRule()

    private fun prefs() = RuntimeEnvironment.getApplication().getSharedPreferences("pip-frame-test", 0)

    @Test
    fun `a dragged picture in picture stays on screen, right of and above where it starts`() {
        val room = Size(800f, 400f)
        assertEquals(Offset(300f, -200f), pipWithin(Offset(300f, -200f), room))
        assertEquals("not past the start edge or below the bottom", Offset(0f, 0f), pipWithin(Offset(-50f, 30f), room))
        assertEquals("not off the far edge or into the status row", Offset(800f, -400f), pipWithin(Offset(5000f, -5000f), room))
        assertEquals("a window smaller than the picture keeps it home", Offset(0f, 0f), pipWithin(Offset(100f, -100f), Size(-10f, -10f)))
    }

    @Test
    fun `the place is remembered as two numbers and anything else starts it home`() {
        assertEquals(Offset(300f, -200f), storedPipPlace("300,-200"))
        assertEquals(Offset.Zero, storedPipPlace(null))
        assertEquals(Offset.Zero, storedPipPlace("300"))
        assertEquals(Offset.Zero, storedPipPlace("a,b"))
    }

    @Test
    fun `the picture is kept between a small thumbnail and most of the screen, and a stored size outside that is brought back in`() {
        val base = Size(320f, 180f)
        val largest = pipLargestScale(base, Size(1200f, 600f))
        assertEquals("seven tenths of the width, unless the height runs out first", 2.625f, largest, 0.001f)
        assertEquals(2.625f, pipScaleWithin(10f, largest), 0.001f)
        assertEquals("never so small it cannot be found or grabbed", 0.6f, pipScaleWithin(0.1f, largest))
        assertEquals("a window too small to grow it in still leaves the smallest size", 0.6f, pipScaleWithin(1.2f, pipLargestScale(base, Size(100f, 50f))))
        assertEquals(Size(480f, 270f), scaledPip(base, 1.5f))
        assertEquals(1.5f, storedPipScale("1.5"))
        assertEquals(1f, storedPipScale(null))
        assertEquals(1f, storedPipScale("nope"))
        assertEquals(1f, storedPipScale("0"))
    }

    @Test
    fun `where the picture was dropped and how big it was pinched come back after a restart`() {
        prefs().edit().clear().commit()
        val pip = PipFrameState(prefs())
        assertEquals(Offset.Zero to 1f, pip.placed to pip.scale)
        pip.adjust(Offset(120f, -40f), 1.5f)
        assertTrue(pip.adjusting)
        pip.keep(pip.placed, pip.scale)
        assertFalse(pip.adjusting)
        val restarted = PipFrameState(prefs())
        assertEquals(Offset(120f, -40f) to 1.5f, restarted.placed to restarted.scale)
    }

    @Test
    fun `a drag moves the picture and never swaps the views, a pinch sizes it, and a tap still swaps`() {
        val moves = mutableListOf<Offset>()
        val zooms = mutableListOf<Float>()
        val taps = mutableListOf<Unit>()
        val ends = mutableListOf<Unit>()
        rule.setContent {
            Box(Modifier.size(300.dp)) {
                Box(Modifier.size(200.dp).testTag("pip").pipGestures(onAdjust = { moved, zoom -> moves += moved; zooms += zoom }, onEnd = { ends += Unit }).clickable { taps += Unit })
            }
        }
        rule.onNodeWithTag("pip").performClick()
        assertEquals(1 to 0, taps.size to moves.size)
        rule.onNodeWithTag("pip").performTouchInput { swipe(center, center + Offset(90f, -60f)) }
        val dragged = moves.fold(Offset.Zero, Offset::plus)
        assertEquals("the whole way the finger went, the slop included", Offset(90f, -60f), dragged)
        assertEquals("the release after a drag is not a tap", 1 to 1, taps.size to ends.size)
        rule.onNodeWithTag("pip").performTouchInput { pinch(center - Offset(20f, 0f), center - Offset(60f, 0f), center + Offset(20f, 0f), center + Offset(60f, 0f)) }
        assertEquals(3f, zooms.fold(1f, Float::times), 0.05f)
        assertEquals(1 to 2, taps.size to ends.size)
    }
}
