package one.aircast.android.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class OverlayEditBarUiTest {
    @get:Rule
    val rule = createComposeRule()

    private val layout = OverlayLayoutState(RuntimeEnvironment.getApplication().getSharedPreferences("overlay-edit-bar-test", 0).also { it.edit().clear().commit() })

    @Before
    fun detachTheCore() {
        Qgc.sendWatch = {}
        Qgc.forgetWatchesForTest()
    }

    @After
    fun clearTheViews() {
        Qgc.publishForTest(FLY_STATE, null)
    }

    private fun flying() = Qgc.publishForTest(FLY_STATE, JSONObject("""{"kind":"object","class":"FlyState","connected":true,"armed":true,"flying":true}"""))

    private fun show(content: @androidx.compose.runtime.Composable () -> Unit) = rule.setContent {
        MaterialTheme { CompositionLocalProvider(LocalFlyScreenState provides FlyScreenState(layout)) { content() } }
    }

    @Test
    fun `on a phone held upright the edit bar keeps Done and every option whole on screen`() {
        layout.editing = true
        show { Box(Modifier.width(PHONE_WIDTH)) { OverlayEditBar() } }
        listOf("Readout size: Default", "Battery bar: Shown", "Reset positions and hidden items", "Done").forEach { label ->
            val node = rule.onNodeWithText(label).assertIsDisplayed()
            assertTrue("$label ends inside the bar", node.getBoundsInRoot().right <= PHONE_WIDTH)
            assertEquals("$label reads on one line", 1, lineCount(node))
        }
    }

    @Test
    @Config(qualifiers = "w960dp-h540dp-land")
    fun `on a wide screen Done keeps to the end of the bar, even on a line of its own`() {
        layout.editing = true
        show { Box(Modifier.width(WIDE_BAR)) { OverlayEditBar() } }
        val done = rule.onNodeWithText("Done").getBoundsInRoot()
        assertTrue("Done ends at the bar's end, not alone at its start", done.right > WIDE_BAR - DONE_END_SLACK)
    }

    @Test
    fun `in flight the edit bar says why, and its Done ends editing`() {
        flying()
        layout.editing = true
        show { OverlayEditBar() }
        rule.onNodeWithText(LAYOUT_EDITING_IN_FLIGHT).assertIsDisplayed()
        rule.onNodeWithText("Done").performClick()
        assertFalse(layout.editing)
    }

    @Test
    fun `on the ground the edit bar does not mention flying`() {
        layout.editing = true
        show { OverlayEditBar() }
        rule.onNodeWithText(LAYOUT_EDITING_IN_FLIGHT).assertDoesNotExist()
    }

    @Test
    fun `while a flight decision is open the edit bar steps aside`() {
        layout.editing = true
        val flyScreen = FlyScreenState(layout).apply { guidedPanels = 1 }
        rule.setContent { MaterialTheme { CompositionLocalProvider(LocalFlyScreenState provides flyScreen) { OverlayEditBar() } } }
        rule.onNodeWithText("Done").assertDoesNotExist()
    }

    @Test
    fun `the battery bar hides and shows from the edit bar`() {
        layout.editing = true
        show { OverlayEditBar() }
        rule.onNodeWithText("Battery bar: Shown").performClick()
        assertTrue(BATTERY_BAR_KEY in layout.hidden)
        rule.onNodeWithText("Battery bar: Hidden").performClick()
        assertFalse(BATTERY_BAR_KEY in layout.hidden)
    }

    @Test
    fun `a reading over the picture is made bigger and smaller from its own badges, and its cross removes it and forgets its size`() {
        val removed = mutableListOf<String>()
        layout.editing = true
        show {
            Box(Modifier.size(300.dp), contentAlignment = androidx.compose.ui.Alignment.Center) {
                LayoutWidget("reading:vehicle/groundSpeed", movable = false, resizable = true, onRemove = { removed += "groundSpeed" }) { Box(Modifier.size(80.dp).testTag("reading")) }
            }
        }
        val normal = rule.onNodeWithTag("reading").fetchSemanticsNode().size.width
        rule.onNodeWithContentDescription("Make bigger").performClick()
        rule.onNodeWithContentDescription("Make bigger").performClick()
        assertEquals(1.5f, layout.sizes["reading:vehicle/groundSpeed"])
        assertEquals((normal * 1.5f).toInt(), rule.onNodeWithTag("reading").fetchSemanticsNode().size.width)
        listOf(1.25f, 1f, 0.85f, 0.7f).map { size ->
            rule.onNodeWithContentDescription("Make smaller").performClick()
            assertEquals(size, layout.sizes["reading:vehicle/groundSpeed"])
        }
        rule.onNodeWithContentDescription("Make smaller").assertDoesNotExist()
        assertEquals(storedSizes(RuntimeEnvironment.getApplication().getSharedPreferences("overlay-edit-bar-test", 0).all), layout.sizes)
        rule.onNodeWithContentDescription("Remove").performClick()
        assertEquals(listOf("groundSpeed"), removed)
        layout.forget(listOf("reading:vehicle/groundSpeed"))
        assertEquals(emptyMap<String, Float>(), layout.sizes)
        layout.resize("headingTape", 2f)
        layout.reset()
        assertEquals(emptyMap<String, Float>(), layout.sizes)
    }

    @Test
    fun `the attitude and compass pair starts hidden, and its row in Choose readings shows and hides it`() {
        show {
            androidx.compose.foundation.layout.Column {
                AttitudeChoice()
                LayoutWidget(ATTITUDE_KEY, name = ATTITUDE_NAME, resizable = true) { AttitudeGauges() }
            }
        }
        rule.onNodeWithContentDescription("Attitude, pitch 0°, roll 0°").assertDoesNotExist()
        rule.onNodeWithText(ATTITUDE_NAME).performClick()
        assertFalse(ATTITUDE_KEY in layout.hidden)
        rule.onNodeWithContentDescription("Attitude, pitch 0°, roll 0°").assertIsDisplayed()
        rule.onNodeWithContentDescription("Heading ").assertIsDisplayed()
        rule.onNodeWithText(ATTITUDE_NAME).performClick()
        assertTrue(ATTITUDE_KEY in layout.hidden)
        rule.onNodeWithContentDescription("Attitude, pitch 0°, roll 0°").assertDoesNotExist()
    }

    @Test
    fun `in flight a hold on a widget starts layout editing`() {
        flying()
        show { LayoutWidget("orbit") { Box(Modifier.size(80.dp).testTag("orbit")) } }
        hold("orbit")
        assertTrue(layout.editing)
    }

    @Test
    fun `a press that slides away is a drag, not a hold, and never starts editing`() {
        show { LayoutWidget("orbit") { Box(Modifier.size(200.dp).testTag("orbit")) } }
        rule.mainClock.autoAdvance = false
        rule.onNodeWithTag("orbit").performTouchInput { down(center) }
        rule.mainClock.advanceTimeBy(100)
        rule.onNodeWithTag("orbit").performTouchInput { moveBy(androidx.compose.ui.geometry.Offset(120f, 0f)) }
        rule.mainClock.advanceTimeBy(HOLD_PAST)
        rule.onNodeWithTag("orbit").performTouchInput { up() }
        assertFalse(layout.editing)
    }

    @Test
    fun `holding a slider inside a widget moves the slider and never starts editing`() {
        show { LayoutWidget("rcControls") { Slider(0.5f, {}, Modifier.width(240.dp).ownsLayoutHold().testTag("slider")) } }
        hold("slider")
        assertFalse(layout.editing)
    }

    private fun hold(tag: String) {
        rule.mainClock.autoAdvance = false
        rule.onNodeWithTag(tag).performTouchInput { down(center) }
        rule.mainClock.advanceTimeBy(HOLD_PAST)
        rule.onNodeWithTag(tag).performTouchInput { up() }
        rule.mainClock.advanceTimeBy(100)
    }

    private fun lineCount(node: SemanticsNodeInteraction): Int {
        val layouts = mutableListOf<TextLayoutResult>()
        node.fetchSemanticsNode().config[SemanticsActions.GetTextLayoutResult].action?.invoke(layouts)
        return layouts.single().lineCount
    }

    private companion object {
        val PHONE_WIDTH = 360.dp
        val WIDE_BAR = 600.dp
        val DONE_END_SLACK = 48.dp
        const val HOLD_PAST = 1000L
    }
}
