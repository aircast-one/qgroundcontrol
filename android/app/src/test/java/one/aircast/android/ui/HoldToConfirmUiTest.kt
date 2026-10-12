package one.aircast.android.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performTouchInput
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class HoldToConfirmUiTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun `holding for the full time confirms once`() {
        var fired by mutableIntStateOf(0)
        rule.setContent { MaterialTheme { HoldToConfirm("Slide to land") { fired++ } } }
        rule.mainClock.autoAdvance = false
        rule.onNodeWithText("Slide to land").performTouchInput { down(center) }
        rule.mainClock.advanceTimeBy(DECK_HOLD_MS.toLong() + 200)
        rule.onNodeWithText("Slide to land").performTouchInput { up() }
        rule.mainClock.advanceTimeBy(100)
        assertEquals(1, fired)
    }

    @Test
    fun `a hold whose action changes mid-press never fires the new action`() {
        var fired by mutableIntStateOf(0)
        var label by mutableStateOf("Slide to take off")
        rule.setContent { MaterialTheme { HoldToConfirm(label) { fired++ } } }
        rule.mainClock.autoAdvance = false
        rule.onNodeWithText("Slide to take off").performTouchInput { down(center) }
        rule.mainClock.advanceTimeBy(DECK_HOLD_MS.toLong() / 2)
        label = "Slide to take off anyway"
        rule.mainClock.advanceTimeBy(DECK_HOLD_MS.toLong() * 2)
        rule.onNodeWithText("Slide to take off anyway").performTouchInput { up() }
        rule.mainClock.advanceTimeBy(100)
        assertEquals("the pilot let go after reading the warning, so nothing is sent", 0, fired)
    }
}
