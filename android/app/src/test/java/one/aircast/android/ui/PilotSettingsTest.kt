package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

private fun parameter(name: String, description: String = "Engineer description") = Fact(
    path = "vehicle.parameterManager.getParameter(-1,$name)",
    name = name,
    description = description,
    units = "m",
    valueString = "30",
    value = 30,
    enumStrings = emptyList(),
    enumIndex = -1,
    isBool = false,
    isString = false,
    readOnly = false,
    minString = "",
    maxString = "",
    minIsDefaultForType = true,
    maxIsDefaultForType = true,
    defaultValueString = "",
)

class PilotSettingsTest {
    private val returnHome = pilotSettings(SettingsGroup.Safety).first { it.label == "Return-to-home altitude" }

    @Test
    fun `a pilot setting uses whichever firmware parameter the aircraft reports, under the pilot's word`() {
        val px4 = firstReported(returnHome) { name -> parameter(name).takeIf { name == "RTL_RETURN_ALT" } }
        val ardupilot = firstReported(returnHome) { name -> parameter(name).takeIf { name == "RTL_ALT" } }
        assertEquals("RTL_RETURN_ALT", px4?.name)
        assertEquals("RTL_ALT", ardupilot?.name)
        assertEquals("Return-to-home altitude", px4?.heading)
    }

    @Test
    fun `the engineer description moves behind help once the row has a pilot label`() {
        assertEquals("Engineer description", firstReported(returnHome) { parameter(it) }?.detail)
    }

    @Test
    fun `nothing shows when the aircraft reports none of the parameters`() {
        assertNull(firstReported(returnHome) { null })
    }

    @Test
    fun `only safety and control carry a pilot list`() {
        assertTrue(pilotSettings(SettingsGroup.Safety).isNotEmpty())
        assertTrue(pilotSettings(SettingsGroup.Control).isNotEmpty())
        assertEquals(emptyList<PilotSetting>(), pilotSettings(SettingsGroup.Camera))
    }

    @Test
    fun `settings rows read in pilot words and unknown rows keep theirs`() {
        assertEquals("Max fly-to distance", pilotWorded(parameter("maxGoToLocationDistance")).heading)
        assertEquals("", pilotWorded(parameter("somethingElse")).shortLabel)
    }

    @Test
    fun `desktop-only setup pages fold away from the ones that open here`() {
        val components = listOf("Airframe", "Sensors", "PID Tuning").mapIndexed { index, name -> SetupComponent(index = index, name = name, needsAttention = false) }
        val (here, desktop) = splitDesktopOnly(components) { it.name == SENSORS }
        assertEquals(listOf(SENSORS), here.map { it.name })
        assertEquals(listOf("Airframe", "PID Tuning"), desktop.map { it.name })
    }

    @Test
    fun `safety lists compass, IMU and gyro in DJI's order with their health`() {
        val view = org.json.JSONObject("""{"class":"Calibration","routines":[
            {"id":"gyro","title":"Gyroscope","status":"Calibrated"},
            {"id":"levelHorizon","title":"Level Horizon","status":""},
            {"id":"compass","title":"Compass","status":"Not calibrated"},
            {"id":"accelerometer","title":"Accelerometer","status":"Calibrated"}]}""")
        val checks = sensorChecks(calibrationState(view))
        assertEquals(listOf("compass", "accelerometer", "gyro"), checks.map { it.id })
        assertEquals(listOf(false, true, true), checks.map(::sensorHealthy))
        assertEquals(emptyList<CalibrationRoutine>(), sensorChecks(null))
    }

    @Test
    fun `failsafe choices read in DJI's words`() {
        assertEquals("Hover", pilotChoice("Hold mode"))
        assertEquals("Return home", pilotChoice("Return mode"))
        assertEquals("Stop motors", pilotChoice("Terminate"))
        assertEquals("Enabled always RTL", pilotChoice("Enabled always RTL"))
    }

    @Test
    fun `search finds a pilot setting by its words and by either firmware's parameter name`() {
        assertEquals(listOf("Return-to-home altitude"), pilotSearchHits("return").map { it.label })
        assertEquals(listOf("Return-to-home altitude"), pilotSearchHits("rtl_alt").map { it.label })
        assertEquals(listOf("Max horizontal speed"), pilotSearchHits("MPC_XY_VEL_MAX").map { it.label })
        assertEquals("Safety \u203a Return to home", pilotSearchHits("return").single().section)
        assertTrue(pilotSearchHits("  ").isEmpty())
    }

    @Test
    fun `offline, safety still lists its sections in DJI's order so the pilot knows they exist`() {
        assertEquals(listOf("Return to home", "Flight protection", "If something goes wrong"), pilotSections(pilotSettings(SettingsGroup.Safety)).keys.toList())
    }
}
