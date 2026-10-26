package one.aircast.android.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class RecordingPillUiTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun `the recording pill stops the recording itself, so a vehicle lost mid-recording cannot strand it`() {
        var stops by mutableIntStateOf(0)
        rule.setContent { MaterialTheme { RecordingPill("Recording route · 123 m") { stops++ } } }
        rule.onNodeWithText("Recording route · 123 m").assertIsDisplayed()
        rule.onNodeWithText("Stop").performClick()
        assertEquals(1, stops)
    }
}
