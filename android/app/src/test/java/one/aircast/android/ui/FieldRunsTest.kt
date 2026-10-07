package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Test

class FieldRunsTest {
    private fun choice(name: String, options: List<String> = listOf("Metric", "Imperial", "SI", "Nautical", "Other"), label: String = name) = Fact(
        path = "settings.$name", name = name, description = "$name long description", units = "", valueString = options.first(),
        value = 0, enumStrings = options, enumIndex = 0, isBool = false, isString = false, readOnly = false, shortLabel = label,
    )

    private fun text(name: String, isString: Boolean = true, units: String = "") = Fact(
        path = "settings.$name", name = name, description = name, units = units, valueString = "", value = "",
        enumStrings = emptyList(), enumIndex = -1, isBool = false, isString = isString, readOnly = false, shortLabel = name,
    )

    @Test
    fun aSettingWithNoEffectYetStillDrawsAsAField() {
        val host = text("host", isString = false).copy(enabled = false, disabledReason = "Has no effect while the ADS-B server connection is off.")
        assertEquals(true, showsAsField(host))
        assertEquals(false, showsAsField(host.copy(readOnly = true)))
    }

    @Test
    fun numbersAndChoicesSitRightOfTheirLabelWhileTextTakesTheWholeRow() {
        assertEquals(true, valueOnTheRight(text("alt", isString = false, units = "m")))
        assertEquals(true, valueOnTheRight(choice("units")))
        assertEquals(false, valueOnTheRight(text("url")))
    }

    @Test
    fun aNumberFieldDropsThePaddedDecimalsButTextIsLeftAlone() {
        assertEquals("300", fieldText(text("d", isString = false).copy(valueString = "300.000")))
        assertEquals("2.5", fieldText(text("d", isString = false).copy(valueString = "2.50")))
        assertEquals("007.10", fieldText(text("s").copy(valueString = "007.10")))
    }

    @Test
    fun aLabelThatNamesAnOptionIsNotShownAsTheSubtitle() {
        val fence = choice("FENCE_ENABLE", listOf("Disabled", "Enabled"), label = "Fence enable/disable").copy(description = "Enabled")
        assertEquals("", fence.detail)
        assertEquals("Fence enable/disable", fence.heading)
        assertEquals("Allows the fence", fence.copy(description = "Allows the fence").detail)
    }

    @Test
    fun aLabelEndingInAColonLosesIt() {
        assertEquals("Return at specified altitude", choice("RTL", label = "Return at specified altitude:").heading)
        assertEquals("Time Offset (seconds)", text("t").copy(description = "Time Offset (seconds):", shortLabel = "").heading)
    }

    @Test
    fun aNoteThatOnlyRestatesTheLabelIsDropped() {
        val multiplier = text("BATT_VOLT_MULT", isString = false).copy(shortLabel = "Voltage Multiplier", description = "Voltage multiplier")
        assertEquals("", multiplier.detail)
        assertEquals("", multiplier.copy(shortLabel = "Battery monitoring", description = "Battery monitor").detail)
        assertEquals("Minimum arming voltage", multiplier.copy(shortLabel = "Required arming voltage", description = "Minimum arming voltage").detail)
        assertEquals("", multiplier.copy(shortLabel = "ArduPilot support host", description = "Ardupilot Support Host name").detail)
        assertEquals("", multiplier.copy(shortLabel = "Color scheme", description = "Application color scheme").detail)
        assertEquals("", multiplier.copy(shortLabel = "RTL loiter time", description = "Loiter time").detail)
        assertEquals("Final land stage altitude", multiplier.copy(shortLabel = "RTL final altitude", description = "Final land stage altitude").detail)
        assertEquals("RC Roll/Pitch feel", multiplier.copy(shortLabel = "Attitude control input time constant", description = "RC Roll/Pitch Feel").detail)
        assertEquals("Only while in Guided mode.", multiplier.copy(shortLabel = "Confirm", description = "Only while in Guided mode.").detail)
    }

    @Test
    fun aRebootNoticeSharedByTheBlockIsSaidOnce() {
        val reboot = text("a").copy(vehicleRebootRequired = true)
        assertEquals("Reboot vehicle for changes to take effect.", sharedRebootNote(listOf(reboot, text("plain"), reboot.copy(name = "b"))))
        assertEquals(null, sharedRebootNote(listOf(reboot, text("plain"))))
        assertEquals(null, sharedRebootNote(listOf(reboot, text("c").copy(qgcRebootRequired = true))))
    }

    @Test
    fun aRowLabelReadsInSentenceCaseKeepingAcronymsAndNames() {
        assertEquals("Mute audio output", sentenceCase("Mute Audio Output"))
        assertEquals("Use preflight checklist", sentenceCase("Use Preflight Checklist"))
        assertEquals("Forward MAVLink to UDP host", sentenceCase("Forward MAVLink To UDP Host"))
        assertEquals("PX4 Pro", sentenceCase("PX4 Pro"))
        assertEquals("3DR Solo (requires restart)", sentenceCase("3DR Solo (requires restart)"))
        assertEquals("Yuneec Mantis G", sentenceCase("Yuneec Mantis G"))
        assertEquals("Herelink Hotspot", sentenceCase("Herelink Hotspot"))
        assertEquals("Mapbox token", sentenceCase("Mapbox Token"))
        assertEquals("Automatically connect to a Pixhawk board", sentenceCase("Automatically connect to a Pixhawk board"))
        assertEquals("Time offset (seconds)", sentenceCase("Time Offset (seconds)"))
        assertEquals("Auto-center throttle", sentenceCase("Auto-Center throttle"))
        assertEquals("Left-handed mode", sentenceCase("Left-Handed mode"))
    }

    @Test
    fun aBlockWhoseFieldsAreAllOffForOneReasonSaysItOnce() {
        val off = text("basicID").copy(enabled = false, disabledReason = "Has no effect while Basic ID broadcast is off.")
        val switch = text("sendBasicID")
        assertEquals("Has no effect while Basic ID broadcast is off.", blockInertNote(listOf(switch, off, off.copy(name = "uaType"))))
        org.junit.Assert.assertNull(blockInertNote(listOf(switch, off)))
    }

    @Test
    fun secondsReadAsS() {
        assertEquals("s", shownUnits("secs"))
        assertEquals("s", shownUnits("Seconds"))
        assertEquals("m/s", shownUnits("m/s"))
    }
}
