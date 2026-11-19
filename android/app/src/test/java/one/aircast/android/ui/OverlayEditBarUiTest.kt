package one.aircast.android.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class OverlayEditBarUiTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun `on a phone held upright the edit bar keeps Done and both options whole on screen`() {
        val prefs = RuntimeEnvironment.getApplication().getSharedPreferences("overlay-edit-bar-test", 0)
        val layout = OverlayLayoutState(prefs).apply { editing = true }
        rule.setContent {
            MaterialTheme {
                CompositionLocalProvider(LocalFlyScreenState provides FlyScreenState(layout)) {
                    Box(Modifier.width(PHONE_WIDTH)) { OverlayEditBar() }
                }
            }
        }
        listOf("Readout size: Default", "Reset positions and hidden items", "Done").forEach { label ->
            val node = rule.onNodeWithText(label).assertIsDisplayed()
            assertTrue("$label ends inside the bar", node.getBoundsInRoot().right <= PHONE_WIDTH)
            assertEquals("$label reads on one line", 1, lineCount(node))
        }
    }

    private fun lineCount(node: SemanticsNodeInteraction): Int {
        val layouts = mutableListOf<TextLayoutResult>()
        node.fetchSemanticsNode().config[SemanticsActions.GetTextLayoutResult].action?.invoke(layouts)
        return layouts.single().lineCount
    }

    private companion object {
        val PHONE_WIDTH = 360.dp
    }
}
