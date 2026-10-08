package one.aircast.android.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.qgcPath

internal data class PilotSetting(val label: String, val parameters: List<String>)

internal const val PILOT_SAFETY_TITLE = "Flight safety"
internal const val PILOT_CONTROL_TITLE = "Flight limits"

internal fun pilotSettings(group: SettingsGroup): List<PilotSetting> = when (group) {
    SettingsGroup.Safety -> listOf(
        PilotSetting("Return-to-home altitude", listOf("RTL_RETURN_ALT", "RTL_ALT")),
        PilotSetting("Max altitude", listOf("GF_MAX_VER_DIST", "FENCE_ALT_MAX")),
        PilotSetting("Max distance", listOf("GF_MAX_HOR_DIST", "FENCE_RADIUS")),
        PilotSetting("Signal lost", listOf("NAV_DLL_ACT", "FS_GCS_ENABLE")),
        PilotSetting("Remote controller lost", listOf("NAV_RCL_ACT", "FS_THR_ENABLE")),
        PilotSetting("Low battery", listOf("COM_LOW_BAT_ACT", "BATT_FS_LOW_ACT")),
    )
    SettingsGroup.Control -> listOf(
        PilotSetting("Max horizontal speed", listOf("MPC_XY_VEL_MAX", "LOIT_SPEED")),
        PilotSetting("Max climb speed", listOf("MPC_Z_VEL_MAX_UP", "PILOT_SPEED_UP")),
        PilotSetting("Max descent speed", listOf("MPC_Z_VEL_MAX_DN", "PILOT_SPEED_DN")),
    )
    else -> emptyList()
}

internal fun pilotSettingsTitle(group: SettingsGroup): String =
    if (group == SettingsGroup.Safety) PILOT_SAFETY_TITLE else PILOT_CONTROL_TITLE

internal fun firstReported(setting: PilotSetting, reported: (String) -> Fact?): Fact? =
    setting.parameters.firstNotNullOfOrNull(reported)?.copy(shortLabel = setting.label)

@Composable
internal fun PilotSettings(group: SettingsGroup) {
    val settings = pilotSettings(group)
    if (settings.isEmpty()) return
    val setup by qgcPath(SETUP)
    val facts by produceState(emptyList<Fact>(), group, setup) {
        value = withContext(Dispatchers.Default) { settings.mapNotNull { firstReported(it, ::parameterFact) } }
    }
    if (facts.isEmpty()) return
    SectionHeader(pilotSettingsTitle(group))
    facts.map { fact -> PilotFactRow(fact) }
}

@Composable
private fun PilotFactRow(shown: Fact) {
    val name = shown.name
    val live by qgcPath(parameterPath(name))
    val fact by produceState(shown, name, live) {
        value = withContext(Dispatchers.Default) { parameterFact(name)?.copy(shortLabel = shown.shortLabel) } ?: shown
    }
    FactRow(fact)
}

private val PILOT_LABELS = mapOf(
    "guidedMinimumAltitude" to "Lowest altitude for takeoff and fly-to",
    "guidedMaximumAltitude" to "Highest altitude for takeoff and fly-to",
    "maxGoToLocationDistance" to "Max fly-to distance",
    "forwardFlightGoToLocationLoiterRad" to "Circle radius after fly-to (planes)",
    "goToLocationRequiresConfirmInGuided" to "Confirm fly-to first",
    "updateHomePosition" to "Home follows this phone",
    "adsbServerConnectEnabled" to "Show nearby aircraft (ADS-B)",
    "adsbServerHostAddress" to "ADS-B server address",
    "adsbServerPort" to "ADS-B server port",
    "displayPresetsTabFirst" to "Open Plan on templates",
    "useConditionGate" to "Use gate commands in survey patterns",
    "takeoffItemNotRequired" to "Missions can start without a takeoff",
    "allowMultipleLandingPatterns" to "Allow several landing paths",
    "vtolTransitionDistance" to "VTOL transition distance",
    "keepMapCenteredOnVehicle" to "Keep the aircraft centred on the map",
    "showAdditionalIndicatorsCompass" to "Extra compass markers",
    "lockNoseUpCompass" to "North-up compass",
    "showObstacleDistanceOverlay" to "Show obstacle distances",
    "showPhotoVideoControl" to "Show the shutter button",
    "showSimpleCameraControl" to "Simple camera trigger",
    "showLogReplayStatusBar" to "Show the log replay bar",
    "requestControlAllowTakeover" to "Let other stations take control",
    "requestControlTimeout" to "Control request timeout",
    "valueDisplay" to "Battery shows",
    "threshold1" to "Battery warning level",
    "threshold2" to "Battery critical level",
    "consolidateMultipleBatteries" to "Combine multiple batteries",
    "streamEnabled" to "Show video",
    "videoFit" to "Video fit",
    "gridLines" to "Grid lines",
    "recordingFormat" to "Recording format",
    "maxVideoSize" to "Storage limit",
    "rtspTimeout" to "Connection timeout",
    "forceVideoDecoder" to "Video decoder",
    "aspectRatio" to "Aspect ratio",
    "defaultMissionItemAltitude" to "Default waypoint altitude",
    "offlineEditingFirmwareClass" to "Plan offline for firmware",
    "offlineEditingVehicleClass" to "Plan offline for aircraft type",
    "offlineEditingCruiseSpeed" to "Planning cruise speed",
    "offlineEditingHoverSpeed" to "Planning hover speed",
    "offlineEditingAscentSpeed" to "Planning climb speed",
    "offlineEditingDescentSpeed" to "Planning descent speed",
    "followTarget" to "Send this phone's location (Follow Me)",
    "enableMultiVehiclePanel" to "Show the multi-aircraft panel",
    "androidDontSaveToSDCard" to "Save data to the SD card",
    "disableAllPersistence" to "Don't save any data",
)

internal fun pilotWorded(fact: Fact): Fact =
    PILOT_LABELS[fact.name]?.let { label -> fact.copy(shortLabel = label) } ?: fact
