package one.aircast.android.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import one.aircast.android.bridge.Qgc
import org.json.JSONObject
import org.junit.After
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment

@RunWith(RobolectricTestRunner::class)
class VehicleSwitchUiTest {
    @get:Rule
    val rule = createComposeRule()

    private fun active(id: Int) = Qgc.publishForTest(one.aircast.map.VEHICLES_VIEW, JSONObject("""{"activeId":$id}"""))

    @Before
    fun detachTheCore() {
        Qgc.sendWatch = {}
        Qgc.forgetWatchesForTest()
    }

    @After
    fun clearTheViews() {
        Qgc.publishForTest(one.aircast.map.VEHICLES_VIEW, null)
        Qgc.publishForTest("view.missionProgress", null)
        Qgc.publishForTest(FLY_STATE, null)
        Qgc.publishForTest(GUIDED_ACTIONS, null)
    }

    @Test
    fun `a skip-waypoint sheet opened for one vehicle closes when another becomes active`() {
        active(1)
        Qgc.publishForTest("view.missionProgress", JSONObject("""{"shown":true,"current":2,"last":5,"canSkip":true,"skipTo":3}"""))
        rule.setContent { MaterialTheme { MissionProgressCard() } }
        rule.onNodeWithText("Skip").performClick()
        rule.onNodeWithText("Set waypoint").assertExists()
        active(2)
        rule.waitForIdle()
        rule.onNodeWithText("Set waypoint").assertDoesNotExist()
    }

    @Test
    fun `a flight action confirm opened for one vehicle never reaches the next active one`() {
        active(1)
        Qgc.publishForTest(FLY_STATE, JSONObject("""{"kind":"object","class":"FlyState","connected":true,"armed":true,"flying":true}"""))
        Qgc.publishForTest(GUIDED_ACTIONS, JSONObject("""{"actions":[{"id":"rtl","title":"Return","offer":"ready","prompt":"Return to the launch position of the vehicle"}]}"""))
        val prefs = RuntimeEnvironment.getApplication().getSharedPreferences("vehicle-switch-test", 0)
        rule.setContent {
            MaterialTheme {
                CompositionLocalProvider(
                    LocalFlyScreenState provides FlyScreenState(OverlayLayoutState(prefs)),
                    LocalAppNavigation provides AppNavigationState(),
                ) { FlightActions() }
            }
        }
        rule.onNodeWithText("Return").performClick()
        rule.onNodeWithText("Return to the launch position of the vehicle", substring = true).assertExists()
        active(2)
        rule.waitForIdle()
        rule.onNodeWithText("Return to the launch position of the vehicle", substring = true).assertDoesNotExist()
    }
}
