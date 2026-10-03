package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Test

class GimbalSettingsTest {
    private fun fact(name: String, value: Any?) = Fact(
        path = "settings.gimbalControllerSettings.$name", name = name, description = name, units = "",
        valueString = value.toString(), value = value, enumStrings = emptyList(), enumIndex = -1,
        isBool = value is Boolean, isString = false, readOnly = false,
    )

    private fun settings(enabled: Boolean, dragging: Boolean) = listOf(
        fact("enableOnScreenControl", enabled), fact("clickAndDrag", dragging), fact("cameraHFov", 70.0), fact("cameraVFov", 70.0),
        fact("cameraSlideSpeed", 30.0), fact("zoomMaxSpeed", 100), fact("zoomMinSpeed", 1), fact("joystickButtonsSpeed", 30),
        fact("showAzimuthIndicatorOnMap", false), fact("toolbarIndicatorShowAzimuth", false), fact("toolbarIndicatorShowAcquireReleaseControl", false),
    )

    @Test
    fun `the on-screen rows follow GimbalIndicator's expanded page`() {
        val tail = listOf("zoomMaxSpeed", "zoomMinSpeed", "joystickButtonsSpeed", "showAzimuthIndicatorOnMap", "toolbarIndicatorShowAzimuth", "toolbarIndicatorShowAcquireReleaseControl")
        assertEquals(listOf("enableOnScreenControl") + tail, gimbalSettingsShown(settings(false, false), true).map { it.name })
        assertEquals(listOf("enableOnScreenControl", "clickAndDrag", "cameraHFov", "cameraVFov") + tail, gimbalSettingsShown(settings(true, false), true).map { it.name })
        assertEquals(listOf("enableOnScreenControl", "clickAndDrag", "cameraSlideSpeed") + tail, gimbalSettingsShown(settings(true, true), true).map { it.name })
    }

    @Test
    fun `joystick buttons speed is read-only without a joystick enabled for the vehicle`() {
        val speed = gimbalSettingsShown(settings(true, false), false).first { it.name == "joystickButtonsSpeed" }
        assertEquals(false, speed.enabled)
        assertEquals(false, joystickButtonsAvailable(org.json.JSONObject("""{"active":null,"vehicle":true,"enabled":true}""")))
        assertEquals(true, joystickButtonsAvailable(org.json.JSONObject("""{"active":"Pad","vehicle":true,"enabled":true}""")))
    }
}
