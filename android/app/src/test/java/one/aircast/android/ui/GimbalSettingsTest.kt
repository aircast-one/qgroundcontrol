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

    private val flyView = listOf(
        SettingsSectionRows("Fly View", "flyViewSettings", "", listOf(SettingsBlock("", listOf(fact("showGimbalOnScreenControl", true))))),
        SettingsSectionRows("Gimbal Controller", "gimbalControllerSettings", "", listOf(
            SettingsBlock("On-Screen Control", listOf(fact("enableOnScreenControl", true), fact("clickAndDrag", false))),
            SettingsBlock("Zoom speed", listOf(fact("zoomMaxSpeed", 100), fact("zoomMinSpeed", 1))),
            SettingsBlock("", listOf(fact("joystickButtonsSpeed", 30), fact("showAzimuthIndicatorOnMap", false))),
        )),
    )

    @Test
    fun `the sheet shows the gimbal section's headed blocks like GimbalIndicator's expanded page`() {
        assertEquals(listOf("On-Screen Control", "Zoom speed", ""), gimbalSettingsBlocks(flyView, true).map { it.title })
        assertEquals(listOf("enableOnScreenControl", "clickAndDrag"), gimbalSettingsBlocks(flyView, true).first().facts.map { it.name })
    }

    @Test
    fun `joystick buttons speed is read-only without a joystick enabled for the vehicle`() {
        val speed = { buttons: Boolean -> gimbalSettingsBlocks(flyView, buttons).flatMap { it.facts }.first { it.name == "joystickButtonsSpeed" } }
        assertEquals(false, speed(false).enabled)
        assertEquals(true, speed(true).enabled)
        assertEquals(false, joystickButtonsAvailable(org.json.JSONObject("""{"active":null,"vehicle":true,"enabled":true}""")))
        assertEquals(true, joystickButtonsAvailable(org.json.JSONObject("""{"active":"Pad","vehicle":true,"enabled":true}""")))
    }
}
